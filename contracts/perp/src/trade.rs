use crate::borrowing::state::{GROUP_OIS, PAIR_OIS};
use crate::borrowing::{
    get_trade_liquidation_price_with_fees, handle_trade_borrowing,
};
use crate::constants::{
    GOV_PRICE_COLLATERAL_INDEX, MAX_OPEN_NEGATIVE_PNL_P, MAX_PNL_P, MAX_SL_P,
};
use crate::error::ContractError;
use crate::fees::calculate_fee_amount;
use crate::fees::state::PENDING_GOV_FEES;
use crate::pairs::state::{
    FEES, GROUPS, ORACLE_ADDRESS, PAIRS, PAIR_CUSTOM_MAX_LEVERAGE,
    STAKING_ADDRESS,
};
use crate::price_impact::{
    add_price_impact_open_interest, get_trade_price_impact,
    remove_price_impact_open_interest,
};
use crate::trading::state::{
    OpenOrderType, PendingOrderType, Trade, TradeInfo, TradeType,
    TradingActivated, COLLATERALS, TRADER_STORED, TRADES, TRADE_INFOS,
    TRADING_ACTIVATED, USER_COUNTERS,
};
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Coin, Decimal, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, Storage, Uint128, WasmQuery,
};
use oracle::contract::OracleQueryMsg;

pub fn open_trade(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    trade: Trade,
    order_type: OpenOrderType,
    max_slippage_p: Decimal,
) -> Result<Response, ContractError> {
    let mut trade = trade.clone();

    let pair = PAIRS
        .load(deps.storage, trade.clone().pair_index)
        .map_err(|_| ContractError::PairNotFound(trade.pair_index))?;

    let base_price = get_token_price(&deps.as_ref(), &pair.oracle_index)?;

    let pair_fees = FEES.load(deps.storage, pair.fee_index)?;
    let group = GROUPS.load(deps.storage, pair.group_index)?;

    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;
    let collateral_price =
        get_collateral_price(&deps.as_ref(), &trade.collateral_index.clone())?;

    let position_size_usd =
        get_usd_normalized_value(collateral_price, position_size_collateral)?;

    trade.open_price = base_price;

    // trade collateral usd value need to be >= 5x min trade fee usd
    // (collateral left after trade opened >= 80%)
    if position_size_usd.checked_div(trade.leverage)?
        < pair_fees.get_min_fee_usd()?.checked_mul(5_u16.into())?
    {
        return Err(ContractError::InsufficientCollateral);
    }

    if trade.leverage < group.min_leverage || trade.leverage > group.max_leverage
    {
        return Err(ContractError::InvalidLeverage);
    }

    if let Some(pair_max_leverage) =
        PAIR_CUSTOM_MAX_LEVERAGE.may_load(deps.storage, trade.pair_index)?
    {
        if trade.leverage > pair_max_leverage {
            return Err(ContractError::InvalidLeverage);
        }
    }

    let msgs: Vec<BankMsg>;
    if trade.trade_type != TradeType::Trade {
        // limit orders are stored as such in the same state, we just don't
        // update the open interest since they are not "live"
        msgs = store_trade(
            deps,
            env.clone(),
            trade.clone(),
            None,
            Some(max_slippage_p),
        )?;
    } else {
        validate_trade(
            deps.as_ref(),
            env.clone(),
            trade.clone(),
            position_size_usd,
            base_price,
            base_price,
            max_slippage_p,
        )?;
        let height = env.clone().block.height;
        let time = env.block.time;

        let trade_info = TradeInfo {
            created_block: height,
            tp_last_updated_block: height,
            sl_last_updated_block: height,
            last_oi_update_ts: time,
            max_slippage_p,
            collateral_price_usd: collateral_price,
        };

        msgs = register_trade(deps, env, trade.clone(), trade_info, order_type)?;
    }

    register_potential_referrer(&info, &trade)?;
    Ok(Response::new()
        .add_attribute("action", "open_trade")
        .add_messages(msgs))
}

// Validate the trade and store it as a trade
fn register_trade(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    trade_info: TradeInfo,
    order_type: OpenOrderType,
) -> Result<Vec<BankMsg>, ContractError> {
    let mut final_trade = trade.clone();
    let (msgs, fees) = process_opening_fees(
        deps,
        env.clone(),
        trade.clone(),
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
        order_type,
    )?;
    final_trade.collateral_amount -= fees;
    store_trade(deps, env, final_trade, Some(trade_info), None)?;

    Ok(msgs)
}

fn process_opening_fees(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    position_size_collateral: Uint128,
    order_type: OpenOrderType,
) -> Result<(Vec<BankMsg>, Uint128), ContractError> {
    let gov_price_collateral =
        get_token_price(&deps.as_ref(), &GOV_PRICE_COLLATERAL_INDEX)?;

    let position_size_collateral = get_position_size_collateral_basis(
        &deps.as_ref(),
        &trade.collateral_index,
        &trade.pair_index,
        position_size_collateral,
    )?;

    // todo: fee tier points
    let mut total_fees_collateral = Uint128::zero();
    let reward1 = Uint128::zero();
    if false {
        // handle referral fees
        total_fees_collateral += reward1;
    }

    let gov_fee_collateral: Uint128 = distribute_gov_fee_collateral(
        deps,
        env.clone(),
        &trade.collateral_index,
        trade.user.clone(),
        &trade.pair_index,
        position_size_collateral,
        gov_price_collateral,
        Decimal::from_ratio(reward1, 2_u64).to_uint_floor(),
    )?;

    let reward2 = calculate_fee_amount(
        &deps.as_ref(),
        env,
        &trade.user,
        Decimal::from_atomics(position_size_collateral, 0)?
            .checked_mul(pair_trigger_order_fee(
                &deps.as_ref(),
                trade.pair_index,
            )?)?
            .to_uint_floor(),
    )?;

    total_fees_collateral +=
        gov_fee_collateral.checked_mul(2_u64.into())? + reward2;

    let mut msgs: Vec<BankMsg> = vec![];
    let reward3: Uint128;
    if order_type != OpenOrderType::MARKET {
        reward3 =
            Decimal::from_ratio(reward2.checked_mul(2_u64.into())?, 10_u64)
                .to_uint_floor();

        msgs.extend(distribute_trigger_fee_gov(
            &trade.user,
            trade.collateral_index,
            reward3,
            gov_price_collateral,
        )?)
    } else {
        reward3 = Uint128::zero();
    }

    msgs.extend(distribute_staking_fee_collateral(
        &deps.as_ref(),
        trade.collateral_index,
        &trade.user,
        gov_fee_collateral + reward2 - reward3,
    )?);

    Ok((msgs, total_fees_collateral))
}

fn distribute_staking_fee_collateral(
    deps: &Deps,
    collateral_index: u64,
    _user: &Addr,
    reward3: Uint128,
) -> Result<Vec<BankMsg>, ContractError> {
    let message = BankMsg::Send {
        to_address: STAKING_ADDRESS.load(deps.storage)?.to_string(),
        amount: vec![Coin::new(
            reward3,
            COLLATERALS.load(deps.storage, collateral_index)?,
        )],
    };

    Ok(vec![message])
}

fn distribute_gov_fee_collateral(
    deps: &mut DepsMut,
    env: Env,
    collateral_index: &u64,
    user: Addr,
    pair_index: &u64,
    position_size_collateral: Uint128,
    gov_price_collateral: Decimal,
    referral_fee_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let gov_fee_collateral = get_gov_fee_collateral(
        &deps.as_ref(),
        env,
        user.clone(),
        *pair_index,
        position_size_collateral,
        gov_price_collateral,
    )? - referral_fee_collateral;

    distribute_exact_gov_fee_collateral(
        deps,
        *collateral_index,
        user,
        gov_fee_collateral,
    )
}

fn get_gov_fee_collateral(
    deps: &Deps,
    env: Env,
    user: Addr,
    pair_index: u64,
    position_size_collateral: Uint128,
    _gov_price_collateral: Decimal,
) -> Result<Uint128, ContractError> {
    let pair = PAIRS.load(deps.storage, pair_index)?;
    let fee = FEES.load(deps.storage, pair.fee_index)?;

    calculate_fee_amount(
        deps,
        env,
        &user,
        Decimal::from_atomics(position_size_collateral, 0)?
            .checked_mul(fee.open_fee_p)?
            .to_uint_floor(),
    )
}

fn distribute_exact_gov_fee_collateral(
    deps: &mut DepsMut,
    collateral_index: u64,
    _user: Addr,
    gov_fee_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let mut pending_gov_fees =
        PENDING_GOV_FEES.load(deps.as_ref().storage, collateral_index)?;
    pending_gov_fees += gov_fee_collateral;
    PENDING_GOV_FEES.save(deps.storage, collateral_index, &pending_gov_fees)?;
    Ok(pending_gov_fees)
}

fn distribute_trigger_fee_gov(
    _user: &Addr,           // for events
    _collateral_index: u64, // for events
    trigger_fee_collateral: Uint128,
    gov_price_collateral: Decimal,
) -> Result<Vec<BankMsg>, ContractError> {
    let trigger_fee_gov = gov_price_collateral
        .checked_div(Decimal::from_atomics(trigger_fee_collateral, 0)?)?
        .to_uint_floor();

    Ok(distribute_trigger_reward(trigger_fee_gov))
}

fn distribute_trigger_reward(_trigger_fee_gov: Uint128) -> Vec<BankMsg> {
    // todo - do we want to reward oracles?
    Vec::new()
}

fn pair_trigger_order_fee(
    deps: &Deps,
    pair_index: u64,
) -> Result<Decimal, ContractError> {
    let pair = PAIRS.load(deps.storage, pair_index)?;
    let fee = FEES.load(deps.storage, pair.fee_index)?;

    Ok(fee.trigger_order_fee_p)
}

fn get_position_size_collateral_basis(
    deps: &Deps,
    collateral_index: &u64,
    pair_index: &u64,
    position_size_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let pair = PAIRS.load(deps.storage, *pair_index)?;
    let min_fee = FEES.load(deps.storage, pair.fee_index)?.get_min_fee_usd()?;
    let collateral_price = get_collateral_price_usd(deps, *collateral_index)?;

    let min_fee_collateral = Decimal::from_atomics(min_fee, 0)?
        .checked_div(collateral_price)?
        .to_uint_floor();

    Ok(Uint128::max(position_size_collateral, min_fee_collateral))
}

pub fn get_token_price(
    deps: &Deps,
    oracle_index: &u64,
) -> Result<Decimal, ContractError> {
    let query_msg = OracleQueryMsg::GetPrice {
        index: *oracle_index,
    };
    let request: WasmQuery = WasmQuery::Smart {
        contract_addr: ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        msg: to_json_binary(&query_msg)?,
    };

    let response: Decimal = deps.querier.query(&QueryRequest::Wasm(request))?;
    Ok(response)
}

pub fn get_collateral_price(
    deps: &Deps,
    oracle_index: &u64,
) -> Result<Decimal, ContractError> {
    let query_msg = OracleQueryMsg::GetCollateralPrice {
        index: *oracle_index,
    };
    let request: WasmQuery = WasmQuery::Smart {
        contract_addr: ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        msg: to_json_binary(&query_msg)?,
    };

    let response: Decimal = deps.querier.query(&QueryRequest::Wasm(request))?;
    Ok(response)
}

fn register_potential_referrer(
    _info: &MessageInfo,
    _trade: &Trade,
) -> Result<(), ContractError> {
    todo!()
}

fn store_trade(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    trade_info: Option<TradeInfo>,
    max_slippage_p: Option<Decimal>,
) -> Result<Vec<BankMsg>, ContractError> {
    let mut trade = trade.clone();

    let counter = USER_COUNTERS
        .load(deps.storage, trade.user.clone())
        .unwrap_or(0_u64);

    // Create or update trade_info
    let trade_info = match trade_info {
        Some(info) => info,
        None => {
            let max_slippage_p = match max_slippage_p {
                Some(value) => value,
                None => return Err(ContractError::InvalidMaxSlippage),
            };
            TradeInfo {
                created_block: env.block.height,
                tp_last_updated_block: env.block.height,
                sl_last_updated_block: env.block.height,
                last_oi_update_ts: env.block.time,
                max_slippage_p,
                collateral_price_usd: get_collateral_price_usd(
                    &deps.as_ref(),
                    trade.collateral_index,
                )?,
            }
        }
    };

    trade.is_open = true;
    trade.tp = limit_tp_distance(
        trade.open_price,
        trade.leverage,
        trade.tp,
        trade.long,
    )?;
    trade.sl = limit_sl_distance(
        trade.open_price,
        trade.leverage,
        trade.sl,
        trade.long,
    )?;
    trade.index = counter;

    TRADES.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade.clone(),
    )?;
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade_info,
    )?;
    TRADER_STORED.save(deps.storage, trade.user.clone(), &true)?;
    USER_COUNTERS.save(deps.storage, trade.user.clone(), &(counter + 1))?;

    if trade.trade_type == TradeType::Trade {
        add_trade_oi_collateral(env, deps, trade, trade_info)?;
    }
    Ok(vec![])
}

fn add_trade_oi_collateral(
    env: Env,
    deps: &mut DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
) -> Result<(), ContractError> {
    add_oi_collateral(
        env,
        deps,
        trade.clone(),
        trade_info,
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
    )
}

fn add_oi_collateral(
    env: Env,
    deps: &mut DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    handle_trade_borrowing(
        env.clone(),
        trade.user.clone(),
        trade.index,
        deps.storage,
        trade.collateral_index,
        trade.pair_index,
        position_collateral,
        true,
        trade.long,
    )?;
    add_price_impact_open_interest(
        deps,
        env.clone(),
        trade,
        trade_info,
        position_collateral,
    )?;
    Ok(())
}

fn remove_trade_oi_collateral(
    env: Env,
    deps: &mut DepsMut,
    trade: Trade,
) -> Result<(), ContractError> {
    remove_oi_collateral(
        deps,
        env,
        trade.clone(),
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
    )
}

fn remove_oi_collateral(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    handle_trade_borrowing(
        env.clone(),
        trade.user.clone(),
        trade.index,
        deps.storage,
        trade.collateral_index,
        trade.pair_index,
        position_collateral,
        false,
        trade.long,
    )?;
    remove_price_impact_open_interest(
        deps,
        env.clone(),
        trade,
        position_collateral,
    )?;

    Ok(())
}

fn validate_trade(
    deps: Deps,
    env: Env,
    trade: Trade,
    position_size_usd: Uint128,
    execution_price: Decimal,
    market_price: Decimal,
    max_slippage_p: Decimal,
) -> Result<(Decimal, Decimal), ContractError> {
    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;

    let spread_p = PAIRS.load(deps.storage, trade.pair_index)?.spread_p;

    if market_price.is_zero() {
        return Err(ContractError::TradeInvalid);
    }

    let (price_impact_p, price_after_impact) = get_trade_price_impact(
        deps.storage,
        env,
        get_market_execution_price(execution_price, spread_p, trade.long),
        trade.pair_index,
        trade.long,
        position_size_usd,
    )?;

    let max_slippage = price_after_impact * max_slippage_p;
    if trade.long && market_price > price_after_impact + max_slippage {
        return Err(ContractError::TradeInvalid);
    }

    if !trade.long && market_price < price_after_impact - max_slippage {
        return Err(ContractError::TradeInvalid);
    }

    if !trade.tp.is_zero()
        && ((trade.long && market_price >= trade.tp)
            || (!trade.long && market_price <= trade.tp))
    {
        return Err(ContractError::InvalidTpSl);
    }

    if !trade.sl.is_zero()
        && ((trade.long && market_price <= trade.sl)
            || (!trade.long && market_price >= trade.sl))
    {
        return Err(ContractError::InvalidTpSl);
    }

    let group_index = PAIRS.load(deps.storage, trade.pair_index)?.group_index;

    within_exposure_limits(
        deps.storage,
        trade.pair_index,
        group_index,
        trade.collateral_index,
        trade.long,
        position_size_collateral,
    )?;

    if price_impact_p.checked_mul(Decimal::from_atomics(trade.leverage, 0)?)?
        > MAX_OPEN_NEGATIVE_PNL_P
    {
        return Err(ContractError::PriceImpactTooHigh);
    }

    Ok((price_impact_p, price_after_impact))
}

fn get_market_execution_price(
    price: Decimal,
    spread_p: Decimal,
    long: bool,
) -> Decimal {
    let price_diff = price.checked_mul(spread_p).unwrap();
    if long {
        price.checked_add(price_diff).unwrap()
    } else {
        price.checked_sub(price_diff).unwrap()
    }
}

fn within_exposure_limits(
    storage: &dyn Storage,
    pair_index: u64,
    group_index: u64,
    collateral_index: u64,
    long: bool,
    position_size_collateral: Uint128,
) -> Result<(), ContractError> {
    let group_ois = GROUP_OIS.load(storage, (collateral_index, group_index))?;
    let pair_ois = PAIR_OIS.load(storage, (collateral_index, pair_index))?;

    let pair_oi_collateral = if long { pair_ois.long } else { pair_ois.short };
    let group_oi_collateral = if long {
        group_ois.long
    } else {
        group_ois.short
    };

    if !(position_size_collateral + pair_oi_collateral <= pair_ois.max
        && position_size_collateral + group_oi_collateral <= group_ois.max)
    {
        return Err(ContractError::ExposureLimitReached);
    }
    Ok(())
}

fn get_usd_normalized_value(
    collateral_price: Decimal,
    collateral_value: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(collateral_price
        .checked_mul(Decimal::from_atomics(collateral_value.u128(), 0)?)?
        .to_uint_floor())
}

fn get_collateral_price_usd(
    deps: &Deps,
    collateral_index: u64,
) -> Result<Decimal, ContractError> {
    get_collateral_price(deps, &collateral_index)
}

fn get_position_size_collateral(
    collateral_amount: Uint128,
    leverage: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(leverage.checked_mul(collateral_amount)?)
}

fn limit_tp_distance(
    open_price: Decimal,
    leverage: Uint128,
    tp: Decimal,
    long: bool,
) -> Result<Decimal, ContractError> {
    if tp.is_zero()
        || get_pnl_percent(open_price, tp, long, leverage)? == MAX_PNL_P
    {
        let open_price = open_price;
        let tp_diff = (open_price * MAX_PNL_P)
            .checked_div(Decimal::from_atomics(leverage, 0)?)?;
        let new_tp = if long {
            open_price + tp_diff
        } else if tp_diff <= open_price {
            open_price - tp_diff
        } else {
            Decimal::zero()
        };
        return Ok(new_tp);
    }
    Ok(tp)
}

fn get_pnl_percent(
    open_price: Decimal,
    current_price: Decimal,
    long: bool,
    leverage: Uint128,
) -> Result<Decimal, ContractError> {
    if !open_price.is_zero() {
        let pnl = if long {
            current_price.checked_sub(open_price)?
        } else {
            open_price.checked_sub(current_price)?
        };

        let pnl_percent = pnl.checked_div(open_price)?;
        let leverage = Decimal::from_atomics(leverage, 0)?;
        let pnl_percent = pnl_percent.checked_mul(leverage)?;

        return Ok(Decimal::max(pnl_percent, MAX_PNL_P));
    }
    Ok(Decimal::zero())
}

fn limit_sl_distance(
    open_price: Decimal,
    leverage: Uint128,
    sl: Decimal,
    long: bool,
) -> Result<Decimal, ContractError> {
    if sl > Decimal::zero()
        && get_pnl_percent(open_price, sl, long, leverage)? < MAX_SL_P
    {
        let open_price = open_price;
        let sl_diff = (open_price * MAX_SL_P)
            .checked_div(Decimal::from_atomics(leverage, 0)?)?;
        let new_sl = if long {
            open_price.checked_sub(sl_diff)?
        } else {
            open_price.checked_add(sl_diff)?
        };
        Ok(new_sl)
    } else {
        Ok(sl)
    }
}

pub fn close_trade(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    index: u64,
) -> Result<Response, ContractError> {
    let mut trade = TRADES.load(deps.storage, (info.sender.clone(), index))?;

    if !trade.is_open {
        return Err(ContractError::TradeClosed);
    }

    trade.is_open = false;
    let counter = USER_COUNTERS.load(deps.storage, info.sender.clone())?;
    USER_COUNTERS.save(deps.storage, info.sender.clone(), &(counter - 1))?;
    TRADES.save(deps.storage, (info.sender.clone(), index), &trade)?;

    if trade.trade_type == TradeType::Trade {
        remove_trade_oi_collateral(env, deps, trade)?;
        Ok(Response::new().add_attribute("action", "close_trade"))
    } else {
        Ok(Response::new()
            .add_attribute("action", "cance_open_order")
            .add_message(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin::new(
                    trade.collateral_amount,
                    COLLATERALS.load(deps.storage, trade.collateral_index)?,
                )],
            }))
    }
}

pub fn update_open_order(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    index: u64,
    price: Decimal,
    tp: Decimal,
    sl: Decimal,
    slippage_p: Decimal,
) -> Result<Response, ContractError> {
    let trade = TRADES.load(deps.storage, (info.sender.clone(), index))?;
    update_open_order_details(deps, env, trade, price, tp, sl, slippage_p)?;
    Ok(Response::new().add_attribute("action", "update_open_order"))
}

fn update_open_order_details(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    price: Decimal,
    tp: Decimal,
    sl: Decimal,
    slippage_p: Decimal,
) -> Result<(), ContractError> {
    let mut trade = trade.clone();
    let mut trade_info =
        TRADE_INFOS.load(deps.storage, (trade.user.clone(), trade.index))?;

    if !trade.is_open {
        return Err(ContractError::TradeClosed);
    }
    if trade.trade_type == TradeType::Trade {
        return Err(ContractError::InvalidTradeType);
    }
    if price.is_zero() {
        return Err(ContractError::TradeInvalid);
    }
    if !tp.is_zero() && (trade.long && price >= tp || !trade.long && price <= tp)
    {
        return Err(ContractError::InvalidTpSl);
    }
    if !sl.is_zero() && (trade.long && price <= sl || !trade.long && price >= sl)
    {
        return Err(ContractError::InvalidTpSl);
    }
    if slippage_p.is_zero() {
        return Err(ContractError::InvalidMaxSlippage);
    }

    trade.tp = limit_tp_distance(price, trade.leverage, tp, trade.long)?;
    trade.sl = limit_sl_distance(price, trade.leverage, sl, trade.long)?;

    trade.open_price = price;

    trade_info.max_slippage_p = slippage_p;
    trade_info.created_block = env.block.height;
    trade_info.tp_last_updated_block = env.block.height;
    trade_info.sl_last_updated_block = env.block.height;

    TRADES.save(deps.storage, (trade.user.clone(), trade.pair_index), &trade)?;
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade_info,
    )?;
    Ok(())
}

pub fn cancel_open_order(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    index: u64,
) -> Result<Response, ContractError> {
    close_trade(deps, env, info, index)
}

pub fn update_tp(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    index: u64,
    new_tp: Decimal,
) -> Result<Response, ContractError> {
    let mut trade = TRADES.load(deps.storage, (info.sender.clone(), index))?;
    let mut trade_info =
        TRADE_INFOS.load(deps.storage, (trade.user.clone(), trade.index))?;

    if !trade.is_open {
        return Err(ContractError::TradeClosed);
    }
    if trade.trade_type != TradeType::Trade {
        return Err(ContractError::InvalidTradeType);
    }

    let new_tp =
        limit_tp_distance(trade.open_price, trade.leverage, new_tp, trade.long)?;

    trade.tp = new_tp;
    trade_info.tp_last_updated_block = env.block.height;

    TRADES.save(deps.storage, (trade.user.clone(), trade.pair_index), &trade)?;
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade_info,
    )?;
    Ok(Response::new().add_attribute("action", "update_tp"))
}

pub fn update_sl(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    index: u64,
    new_sl: Decimal,
) -> Result<Response, ContractError> {
    let mut trade = TRADES.load(deps.storage, (info.sender.clone(), index))?;
    let mut trade_info =
        TRADE_INFOS.load(deps.storage, (trade.user.clone(), trade.index))?;

    if !trade.is_open {
        return Err(ContractError::TradeClosed);
    }
    if trade.trade_type != TradeType::Trade {
        return Err(ContractError::InvalidTradeType);
    }

    let new_sl =
        limit_sl_distance(trade.open_price, trade.leverage, new_sl, trade.long)?;

    trade.sl = new_sl;
    trade_info.sl_last_updated_block = env.block.height;

    TRADES.save(deps.storage, (trade.user.clone(), trade.pair_index), &trade)?;
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade_info,
    )?;
    Ok(Response::new().add_attribute("action", "update_sl"))
}

pub fn trigger_limit_order(
    deps: &mut DepsMut,
    env: Env,
    trader: Addr,
    info: MessageInfo,
    index: u64,
    mut pending_order_type: PendingOrderType,
) -> Result<Response, ContractError> {
    if is_order_type_market(&pending_order_type) {
        return Err(ContractError::InvalidTradeType);
    }

    let is_open_limit = pending_order_type == PendingOrderType::LimitOpen
        || pending_order_type == PendingOrderType::StopOpen;

    let activated = TRADING_ACTIVATED.load(deps.storage)?;

    if is_open_limit && activated != TradingActivated::Activated
        || !is_open_limit && activated == TradingActivated::Paused
    {
        return Err(ContractError::Paused);
    }

    let trade = TRADES.load(deps.storage, (trader.clone(), index))?;
    if !trade.is_open {
        return Err(ContractError::TradeClosed);
    }

    if pending_order_type == PendingOrderType::LiqClose && !trade.sl.is_zero() {
        let liq_price = get_trade_liquidation_price_with_fees(
            &deps.as_ref(),
            env.clone(),
            trade.clone(),
            true,
        )?;

        // if liq price not closer than SL, turn order into a SL
        if (trade.long && liq_price <= trade.sl)
            || (!trade.long && liq_price >= trade.sl)
        {
            pending_order_type = PendingOrderType::SlClose;
        }
    }

    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;

    if is_open_limit {
        let leveraged_pos_usd = get_position_size_collateral(
            trade.collateral_amount,
            trade.leverage,
        )?;

        let (price_impact_p, _) = get_trade_price_impact(
            deps.storage,
            env.clone(),
            Decimal::zero(),
            trade.pair_index,
            trade.long,
            leveraged_pos_usd,
        )?;

        if price_impact_p
            .checked_mul(Decimal::from_atomics(trade.leverage, 0)?)?
            > MAX_OPEN_NEGATIVE_PNL_P
        {
            return Err(ContractError::PriceImpactTooHigh);
        }
    }

    let trigger_price = get_token_price(&deps.as_ref(), &trade.pair_index)?;

    execute_trigger(
        deps,
        env,
        info,
        trigger_price,
        trade,
        pending_order_type,
        position_size_collateral,
    )
}

fn execute_trigger(
    deps: &mut DepsMut,
    env: Env,
    info: MessageInfo,
    trigger_price: Decimal,
    trade: Trade,
    _pending_order_type: PendingOrderType,
    position_size_collateral: Uint128,
) -> Result<Response, ContractError> {
    let mut trade = trade.clone();
    let trade_info =
        TRADE_INFOS.load(deps.storage, (trade.user.clone(), trade.index))?;

    let (_, price_after_impact) = validate_trade(
        deps.as_ref(),
        env.clone(),
        trade.clone(),
        get_usd_normalized_value(
            get_collateral_price_usd(&deps.as_ref(), trade.collateral_index)?,
            position_size_collateral,
        )?,
        trigger_price,
        trigger_price,
        trade_info.max_slippage_p,
    )?;

    let no_hit = if trade.trade_type == TradeType::Stop {
        if trade.long {
            trade.open_price < trigger_price
        } else {
            trade.open_price > trigger_price
        }
    } else {
        false
    };

    if no_hit {
        return Err(ContractError::InvalidTriggerPrice);
    }

    close_trade(deps, env.clone(), info, trade.index)?;

    // register the market trade
    trade.open_price = price_after_impact;
    trade.trade_type = TradeType::Trade;

    register_trade(deps, env.clone(), trade, trade_info, todo!())?;

    Ok(Response::new().add_attribute("action", "trigger_limit_order"))
}

fn is_order_type_market(pending_order_type: &PendingOrderType) -> bool {
    let market_order_types: Vec<PendingOrderType> = vec![
        PendingOrderType::MarketOpen,
        PendingOrderType::MarketClose,
        PendingOrderType::UpdateLeverage,
        PendingOrderType::MarketPartialOpen,
        PendingOrderType::MarketPartialClose,
    ];

    market_order_types.contains(pending_order_type)
}
