use crate::borrowing::handle_trade_borrowing;
use crate::borrowing::state::{GROUP_OIS, PAIR_OIS};
use crate::constants::{
    GOV_PRICE_COLLATERAL_INDEX, MAX_OPEN_NEGATIVE_PNL_P, MAX_PNL_P, MAX_SL_P,
};
use crate::error::ContractError;
use crate::fees::calculate_fee_amount;
use crate::fees::state::PENDING_GOV_FEES;
use crate::pairs::state::{
    FEES, GROUPS, ORACLE_ADDRESS, PAIRS, PAIR_CUSTOM_MAX_LEVERAGE,
};
use crate::price_impact::{
    add_price_impact_open_interest, get_trade_price_impact,
};
use crate::trading::state::{
    LimitOrder, OpenOrderType, Trade, TradeInfo, TradeType, TRADER_STORED,
    TRADES, TRADE_INFOS,
};
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, Response, Storage, Uint128, WasmQuery,
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
        .map_err(|_| ContractError::PairNotFound(trade.pair_index.clone()))?;

    let base_price = get_token_price(&deps.as_ref(), &pair.oracle_index)?;

    let pair_fees = FEES.load(deps.storage, pair.fee_index)?;
    let group = GROUPS.load(deps.storage, pair.group_index)?;

    let position_size_collateral = get_position_size_collateral(
        trade.collateral_amount.clone(),
        trade.leverage.clone(),
    )?;
    let collateral_price =
        get_token_price(&deps.as_ref(), &trade.collateral_index.clone())?;

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
        msgs =
            store_trade(deps, env, trade.clone(), None, Some(max_slippage_p))?;
    } else {
        validate_trade(
            deps.as_ref(),
            trade.clone(),
            position_size_usd,
            base_price.clone(),
            base_price,
            pair.spread_p,
            max_slippage_p.clone(),
        )?;
        let height = env.clone().block.height;
        let time = env.block.time;

        let trade_info = TradeInfo {
            created_block: height,
            tp_last_updated_block: height,
            sl_last_updated_block: height,
            last_oi_update_ts: time,
            max_slippage_p: max_slippage_p,
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
        todo!()
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
        trade.collateral_index,
        &trade.user,
        gov_fee_collateral + reward2 - reward3,
    )?);

    Ok((msgs, total_fees_collateral))
}

fn distribute_staking_fee_collateral(
    _collateral_index: u64,
    _user: &Addr,
    _reward3: Uint128,
) -> Result<Vec<BankMsg>, ContractError> {
    todo!()
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
    _user: &Addr,
    _collateral_index: u64,
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
    return Vec::new();
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
        oracle_index: *oracle_index,
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
    // todo update counters
    let mut trade = trade.clone();

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

    TRADES.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index.clone()),
        &trade.clone(),
    )?;
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index.clone()),
        &trade_info,
    )?;
    TRADER_STORED.save(deps.storage, trade.user.clone(), &true)?;

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
        env,
        deps.storage,
        trade.collateral_index.clone(),
        trade.user.clone(),
        trade.pair_index.clone(),
        position_collateral,
        true,
        trade.long.clone(),
    )?;
    add_price_impact_open_interest(
        &deps,
        trade,
        trade_info,
        position_collateral,
    )?;
    Ok(())
}

fn validate_trade(
    deps: Deps,
    trade: Trade,
    position_size_usd: Uint128,
    execution_price: Decimal,
    market_price: Decimal,
    spread_p: Decimal,
    max_slippage_p: Decimal,
) -> Result<(), ContractError> {
    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;

    if market_price.is_zero() {
        return Err(ContractError::TradeInvalid);
    }

    let (price_impact_p, price_after_impact) = get_trade_price_impact(
        deps.storage,
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

    Ok(())
}

fn get_market_execution_price(
    price: Decimal,
    spread_p: Decimal,
    long: bool,
) -> Decimal {
    let price_diff = price.checked_mul(spread_p).unwrap();
    if long {
        return price.checked_add(price_diff).unwrap();
    } else {
        return price.checked_sub(price_diff).unwrap();
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
    Ok(Uint128::from(
        collateral_price
            .checked_mul(Decimal::from_atomics(collateral_value.u128(), 0)?)?
            .to_uint_floor(),
    ))
}

fn get_collateral_price_usd(
    deps: &Deps,
    collateral_index: u64,
) -> Result<Decimal, ContractError> {
    get_token_price(deps, &collateral_index)
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
        } else {
            if tp_diff <= open_price {
                open_price - tp_diff
            } else {
                Decimal::zero()
            }
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

pub fn trigger_order(
    _deps: Deps,
    _order_id: u64,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn close_trade_market(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _pair_index: u64,
    _index: u64,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn update_open_limit_order(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _pair_index: u64,
    _index: u64,
    _price: Decimal,
    _tp: Decimal,
    _sl: Decimal,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn cancel_open_limit_order(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _pair_index: u64,
    _index: u64,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn update_tp(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _pair_index: u64,
    _index: u64,
    _new_tp: Decimal,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn update_sl(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _pair_index: u64,
    _index: u64,
    _new_sl: Decimal,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn execute_limit_order(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _order_type: LimitOrder,
    _trader: String,
    _pair_index: u64,
    _index: u64,
    _nft_id: u64,
    _nft_type: u8,
) -> Result<Response, ContractError> {
    todo!()
}
