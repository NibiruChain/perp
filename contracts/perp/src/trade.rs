use crate::borrowing::handle_trade_borrowing;
use crate::borrowing::state::{GROUP_OIS, PAIR_OIS};
use crate::constants::{MAX_OPEN_NEGATIVE_PNL_P, MAX_PNL_P, MAX_SL_P};
use crate::error::ContractError;
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
    to_json_binary, BankMsg, Decimal, Decimal256, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, Storage, Uint128, WasmQuery,
};
use oracle::contract::OracleQueryMsg;

pub fn open_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade: Trade,
    order_type: OpenOrderType,
    max_slippage_p: Decimal,
) -> Result<Response, ContractError> {
    let user = info.sender.clone();
    let mut trade = trade.clone();

    let pair = PAIRS
        .load(deps.storage, trade.pair_index)
        .map_err(|_| ContractError::PairNotFound(trade.pair_index))?;

    let base_price = get_token_price(deps, pair.oracle_index)?;
    let pair_fees = FEES.load(deps.storage, pair.fee_index)?;
    let group = GROUPS.load(deps.storage, pair.group_index)?;

    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;
    let collateral_price = get_token_price(deps, trade.collateral_index)?;

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

    if trade.trade_type != TradeType::Trade {
        store_trade(deps, env, trade, None);
    } else {
        validate_trade(
            env,
            deps,
            trade,
            position_size_usd,
            collateral_price,
            base_price,
            pair.spread_p,
        )?;
        let trade_info = TradeInfo {
            created_block: env.block.height,
            tp_last_updated_block: env.block.height,
            sl_last_updated_block: env.block.height,
            last_oi_update_ts: env.block.time,
        };

        register_trade(deps, env, trade.clone(), trade_info, order_type)?;
    }

    register_potential_referrer(&info, &trade)?;
    Ok(Response::new().add_attribute("action", "open_trade"))
}

fn store_order(
    deps: DepsMut,
    env: Env,
    trade: Trade,
) -> Result<(), ContractError> {
    todo!();
}

// Validate the trade and store it as a trade
fn register_trade(
    deps: DepsMut,
    env: Env,
    trade: Trade,
    trade_info: TradeInfo,
    order_type: OpenOrderType,
) -> Result<(), ContractError> {
    let mut final_trade = trade.clone();
    let (msgs, fees) = process_opening_fees(
        deps,
        trade,
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
        order_type,
    )?;
    final_trade.collateral_amount -= fees;
    store_trade(deps, env, final_trade, Some(trade_info));

    Ok(())
}

fn process_opening_fees(
    deps: DepsMut,
    trade: Trade,
    position_size_collateral: Uint128,
    order_type: OpenOrderType,
) -> Result<(Vec<BankMsg>, Uint128), ContractError> {
    // todo
    Ok((vec![], Uint128::zero()))
}

pub fn get_token_price(
    deps: DepsMut,
    oracle_index: u64,
) -> Result<Decimal, ContractError> {
    let query_msg = OracleQueryMsg::GetPrice { oracle_index };
    let request: WasmQuery = WasmQuery::Smart {
        contract_addr: ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        msg: to_json_binary(&query_msg)?,
    };

    let response: Decimal = deps.querier.query(&QueryRequest::Wasm(request))?;
    Ok(response)
}

fn register_potential_referrer(
    info: &MessageInfo,
    trade: &Trade,
) -> Result<(), ContractError> {
    todo!()
}

fn store_trade(
    deps: DepsMut,
    env: Env,
    trade: Trade,
    trade_info: Option<TradeInfo>,
) -> Result<(), ContractError> {
    // todo update counters
    let mut trade = trade.clone();

    let trade_info = trade_info.unwrap_or_else(|| TradeInfo {
        created_block: env.block.height,
        tp_last_updated_block: env.block.height,
        sl_last_updated_block: env.block.height,
        last_oi_update_ts: env.block.time,
    });

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

    TRADES.save(deps.storage, (trade.user.clone(), trade.pair_index), &trade);
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade_info,
    );
    TRADER_STORED.save(deps.storage, trade.user, &true);

    if trade.trade_type == TradeType::Trade {
        add_trade_oi_collateral(env, deps, trade, trade_info)?;
    }
    Ok(())
}

fn add_trade_oi_collateral(
    env: Env,
    deps: DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
) -> Result<(), ContractError> {
    add_oi_collateral(
        env,
        deps,
        trade,
        trade_info,
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
    )
}

fn add_oi_collateral(
    env: Env,
    deps: DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    handle_trade_borrowing(
        env,
        deps.storage,
        trade.collateral_index,
        trade.user,
        trade.pair_index,
        position_collateral,
        true,
        trade.long,
    )?;
    add_price_impact_open_interest(
        deps,
        trade,
        trade_info,
        position_collateral,
    )?;
    Ok(())
}

fn validate_trade(
    env: Env,
    deps: DepsMut,
    trade: Trade,
    position_size_usd: Uint128,
    collateral_price: Decimal,
    base_price: Decimal,
    spread_p: Decimal,
) -> Result<(), ContractError> {
    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;

    if base_price.is_zero() {
        return Err(ContractError::TradeInvalid);
    }

    let max_slippage = trade.wanted_price * trade.max_slippage_p;
    if trade.long && base_price > trade.wanted_price + max_slippage {
        return Err(ContractError::TradeInvalid);
    }

    if !trade.long && base_price < trade.wanted_price - max_slippage {
        return Err(ContractError::TradeInvalid);
    }

    if !trade.tp.is_zero()
        && ((trade.long && base_price >= trade.tp)
            || (!trade.long && base_price <= trade.tp))
    {
        return Err(ContractError::InvalidTpSl);
    }

    if !trade.sl.is_zero()
        && ((trade.long && base_price <= trade.sl)
            || (!trade.long && base_price >= trade.sl))
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

    let (price_impact_p, _) = get_trade_price_impact(
        deps.storage,
        get_market_execution_price(base_price, spread_p, trade.long),
        trade.pair_index,
        trade.long,
        position_size_usd,
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

fn get_collateral_price_usd(collateral_index: u8) -> Decimal {
    todo!()
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
        || get_pnl_percent(open_price, tp, long, leverage) == MAX_PNL_P
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
    tp: Decimal,
    long: bool,
    leverage: Uint128,
) -> Decimal {
    todo!()
}

fn limit_sl_distance(
    open_price: Decimal,
    leverage: Uint128,
    sl: Decimal,
    long: bool,
) -> Result<Decimal, ContractError> {
    if sl > Decimal::zero()
        && get_pnl_percent(open_price, sl, long, leverage) < MAX_SL_P
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
    deps: Deps,
    order_id: u64,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn close_trade_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn update_open_limit_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
    price: Decimal,
    tp: Decimal,
    sl: Decimal,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn cancel_open_limit_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn update_tp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
    new_tp: Decimal,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn update_sl(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
    new_sl: Decimal,
) -> Result<Response, ContractError> {
    todo!()
}

pub fn execute_limit_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    order_type: LimitOrder,
    trader: String,
    pair_index: u64,
    index: u64,
    nft_id: u64,
    nft_type: u8,
) -> Result<Response, ContractError> {
    todo!()
}
