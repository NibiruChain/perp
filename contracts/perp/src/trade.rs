use crate::error::ContractError;
use crate::state::{LimitOrder, OpenLimitOrderType, Trade, STATE};
use crate::util::{get_trade_price_impact, validate_order};
use cosmwasm_std::{
    BankMsg, Coin, Decimal256, DepsMut, Env, MessageInfo, Response, Uint128,
};

pub fn open_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade: Trade,
    order_type: OpenLimitOrderType,
    spread_reduction_id: u64,
    slippage_p: Decimal256,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    if state.is_paused {
        return Err(ContractError::OperationsHalted);
    }

    let spread_reduction: Decimal256;
    if spread_reduction_id > 0 {
        spread_reduction =
            state.spread_reductions_p[(spread_reduction_id - 1) as usize];
    } else {
        spread_reduction = Decimal256::zero()
    }
    validate_order(&state, &info, &trade)?;
    let dec_leverage = Decimal256::from_atomics(trade.leverage, 0).unwrap();

    let price_impact = get_trade_price_impact(
        trade.pair_index,
        trade.buy,
        trade.position_size_nusd.checked_mul(dec_leverage).unwrap(),
    );

    if price_impact.checked_mul(dec_leverage).unwrap()
        >= state.max_negative_pnl_on_open_p
    {
        return Err(ContractError::PriceImpactTooHigh);
    }

    if order_type == OpenLimitOrderType::MARKET {
        state
            .execute_market_order(
                trade,
                info,
                env,
                slippage_p,
                spread_reduction,
                spread_reduction,
                price_impact,
            )
            .unwrap();
    } else {
        state.store_open_limit_order(&info, &trade, spread_reduction_id, &env);
    }

    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "open_trade"))
}

pub fn close_trade_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    match (
        state
            .open_trades
            .get(&(info.sender.clone(), pair_index))
            .and_then(|trades| trades.get(&index)),
        state
            .open_trades_info
            .get(&(info.sender.clone(), pair_index))
            .and_then(|trades| trades.get(&index)),
    ) {
        (Some(trade), Some(trade_info)) => {
            if trade_info.being_market_closed {
                return Err(ContractError::AlreadyBeingClosed);
            }
            if trade.leverage <= 0 {
                return Err(ContractError::InvalidLeverage);
            }

            let new_trade = Trade {
                trader: info.sender.clone(),
                pair_index: pair_index,
                initial_pos_token: Decimal256::zero(),
                position_size_nusd: Decimal256::zero(),
                open_price: Decimal256::zero(),
                buy: false,
                leverage: 0,
                tp: Decimal256::zero(),
                sl: Decimal256::zero(),
            };

            state
                .execute_market_order(
                    new_trade,
                    info,
                    env,
                    Decimal256::zero(),
                    Decimal256::zero(),
                    Decimal256::zero(),
                    Decimal256::zero(),
                )
                .unwrap();
            Ok(Response::new().add_attribute("action", "close_trade_market"))
        }
        _ => return Err(ContractError::TradeDoesNotExist),
    }
}

pub fn update_open_limit_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
    price: Decimal256,
    tp: Decimal256,
    sl: Decimal256,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    match state
        .open_limit_orders
        .get(&(info.sender.clone(), pair_index))
        .and_then(|orders| orders.get(&index))
    {
        Some(order) => {
            let mut order = order.clone();
            if env.block.height < order.block + state.limit_orders_timelock {
                return Err(ContractError::LimitOrderTimelock);
            }
            if !(tp.is_zero()
                || (order.buy && price < tp)
                || (!order.buy && price > tp))
            {
                return Err(ContractError::InvalidTpSl);
            }

            if !(sl.is_zero()
                || (order.buy && price > sl)
                || (!order.buy && price < sl))
            {
                return Err(ContractError::InvalidTpSl);
            }

            order.tp = tp;
            order.sl = sl;

            state
                .open_limit_orders
                .get_mut(&(info.sender.clone(), pair_index))
                .unwrap()
                .insert(index, order);
            STATE.save(deps.storage, &state)?;
            Ok(Response::new()
                .add_attribute("action", "update_open_limit_order"))
        }
        None => return Err(ContractError::LimitOrderDoesNotExist),
    }
}

pub fn cancel_open_limit_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let key = (info.sender.clone(), pair_index);

    // Check if the order exists and extract necessary information
    let order = match state.open_limit_orders.get(&key) {
        Some(orders) => orders.get(&index).cloned(),
        None => None,
    }
    .ok_or(ContractError::LimitOrderDoesNotExist)?;

    if env.block.height < order.block + state.limit_orders_timelock {
        return Err(ContractError::LimitOrderTimelock);
    }

    state.open_limit_orders.entry(key).and_modify(|orders| {
        orders.remove(&index);
    });

    STATE.save(deps.storage, &state)?;

    // Prepare the message to send funds back to the trader
    let msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: "uusd".to_string(),
            amount: Uint128::try_from(order.position_size.to_uint_floor())
                .unwrap(),
        }],
    };

    Ok(Response::new()
        .add_attribute("action", "cancel_open_limit_order")
        .add_message(msg))
}

pub fn update_tp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
    new_tp: Decimal256,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    match (
        state
            .open_trades
            .get(&(info.sender.clone(), pair_index))
            .and_then(|trades| trades.get(&index)),
        state
            .open_trades_info
            .get(&(info.sender.clone(), pair_index))
            .and_then(|trades| trades.get(&index)),
    ) {
        (Some(trade), Some(trade_info)) => {
            let mut trade = trade.clone();
            if env.block.height
                < trade_info.tp_last_updated + state.limit_orders_timelock
            {
                return Err(ContractError::LimitOrderTimelock);
            }

            trade.tp = new_tp;
            state
                .open_trades
                .get_mut(&(info.sender.clone(), pair_index))
                .unwrap()
                .insert(index, trade);

            STATE.save(deps.storage, &state)?;
            Ok(Response::new().add_attribute("action", "update_tp"))
        }
        _ => return Err(ContractError::TradeNotFound),
    }
}

pub fn update_sl(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    index: u64,
    new_sl: Decimal256,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    match (
        state
            .open_trades
            .get(&(info.sender.clone(), pair_index))
            .and_then(|trades| trades.get(&index)),
        state
            .open_trades_info
            .get(&(info.sender.clone(), pair_index))
            .and_then(|trades| trades.get(&index)),
    ) {
        (Some(trade), Some(trade_info)) => {
            let mut trade = trade.clone();
            if env.block.height
                < trade_info.tp_last_updated + state.limit_orders_timelock
            {
                return Err(ContractError::LimitOrderTimelock);
            }

            let max_sl_dist = trade
                .open_price
                .checked_mul(state.max_sl_p)
                .unwrap()
                .checked_div(
                    Decimal256::from_atomics(trade.leverage, 0).unwrap(),
                )
                .unwrap();
            if !(new_sl.is_zero()
                || (trade.buy && new_sl >= trade.open_price - max_sl_dist)
                || (!trade.buy && new_sl <= trade.open_price + max_sl_dist))
            {
                return Err(ContractError::SlTooBig);
            }

            trade.sl = new_sl;
            state
                .open_trades
                .get_mut(&(info.sender.clone(), pair_index))
                .unwrap()
                .insert(index, trade);

            STATE.save(deps.storage, &state)?;
            Ok(Response::new().add_attribute("action", "update_tp"))
        }
        _ => return Err(ContractError::TradeNotFound),
    }
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
