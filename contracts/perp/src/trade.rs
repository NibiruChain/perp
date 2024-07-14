use crate::borrowing::state::{GROUP_OIS, PAIR_OIS};
use crate::constants::MAX_OPEN_NEGATIVE_PNL_P;
use crate::error::ContractError;
use crate::pairs::state::{
    FEES, GROUPS, ORACLE_ADDRESS, PAIRS, PAIR_CUSTOM_MAX_LEVERAGE,
};
use crate::price_impact::get_trade_price_impact;
use crate::trading::state::{
    LimitOrder, OpenOrderType, Trade, TradeInfo, TradeType,
};
use cosmwasm_std::{
    to_json_binary, BankMsg, Coin, Decimal, Decimal256, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Response, Storage, Uint128, WasmQuery,
};
use oracle::contract::OracleQueryMsg;

pub fn open_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade: Trade,
    order_type: OpenOrderType,
) -> Result<Response, ContractError> {
    let user = info.sender.clone();
    let mut trade = trade.clone();

    let pair = PAIRS.load(deps.storage, trade.pair_index)?;
    let base_price = get_token_price(deps, pair.oracle_index)?;
    let pair_fees = FEES.load(deps.storage, pair.fee_index)?;
    let group = GROUPS.load(deps.storage, pair.group_index)?;

    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage);
    let collateral_price = get_token_price(deps, trade.collateral_index)?;

    let position_size_usd =
        get_usd_normalized_value(collateral_price, position_size_collateral)?;

    trade.open_price = base_price;

    // trade collateral usd value need to be >= 5x min trade fee usd (collateral left after trade opened >= 80%)
    if position_size_usd
        .checked_div(Decimal::from_ratio(trade.leverage, Uint128::one()))?
        < pair_fees
            .get_min_fee_usd()?
            .checked_mul(Decimal::from_atomics(5_u32, 0)?)?
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

    if trade.trade_type != TradeType::Market {
        let trade_info = TradeInfo {
            created_block: env.block.height,
            tp_last_updated_block: env.block.height,
            sl_last_updated_block: env.block.height,
            last_oi_update_ts: env.block.time,
        };
        store_order(trade, trade_info)
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

        execute_order(deps, env, trade.clone(), None, order_type)?;
    }

    register_potential_referrer(&info, &trade)?;
    Ok(Response::new().add_attribute("action", "open_trade"))
}

fn store_order(trade: Trade, trade_info: TradeInfo) {
    todo!()
}

fn execute_order(
    deps: DepsMut,
    env: Env,
    trade: Trade,
    trade_info: Option<TradeInfo>,
    order_type: OpenOrderType,
) -> Result<(), ContractError> {
    let mut final_trade = trade.clone();
    final_trade.collateral_amount -= process_opening_fees(
        deps,
        trade,
        get_position_size_collateral(trade.collateral_amount, trade.leverage),
        order_type,
    )?;

    store_trade(deps, final_trade, trade_info);

    Ok(())
}

fn process_opening_fees(
    deps: DepsMut,
    trade: Trade,
    leverage: Uint128,
    order_type: OpenOrderType,
) -> Result<Uint128, ContractError> {
    todo!()
}

fn store_pending_order(trade: Trade, trade_info: TradeInfo) {
    todo!()
}

fn get_token_price(
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

fn execute_market_order(trade: Trade) {
    todo!()
}

fn store_trade(deps: DepsMut, trade: Trade, trade_info: Option<TradeInfo>) {
    todo!()
}

fn validate_trade(
    env: Env,
    deps: DepsMut,
    trade: Trade,
    position_size_usd: Decimal,
    collateral_price: Decimal,
    base_price: Decimal,
    spread_p: Decimal,
) -> Result<(), ContractError> {
    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage);

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
) -> Result<Decimal, ContractError> {
    Ok(collateral_price
        .checked_mul(Decimal::from_atomics(collateral_value, 0)?)?)
}

fn get_collateral_price_usd(collateral_index: u8) -> Decimal {
    todo!()
}

fn get_position_size_collateral(
    collateral_amount: Uint128,
    leverage: u64,
) -> Uint128 {
    return collateral_amount.checked_mul(leverage.into()).unwrap();
}

pub fn trigger_order(
    deps: Deps,
    order_id: u64,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    let pending_order = state
        .pending_orders
        .get(order_id)
        .ok_or(ContractError::OrderNotFound)?;

    let trade = pending_order.trade.clone();
    let trade_info = pending_order.trade_info.clone();

    let position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage);
    let position_size_usd = get_usd_normalized_value(
        trade.collateral_index,
        position_size_collateral,
    );

    state.within_exposure_limit(
        trade.collateral_index,
        trade.pair_index,
        trade.long,
        position_size_collateral,
    )?;

    // trade collateral usd value need to be >= 5x min trade fee usd (collateral left after trade opened >= 80%)
    if position_size_usd / trade.leverage
        < 5 * pair_min_fee_usd(trade.pair_index)
    {
        return Err(ContractError::InsufficientCollateral);
    }

    if (trade.leverage < pair_min_leverage(trade.pair_index)
        || trade.leverage > pair_max_leverage(trade.pair_index))
    {
        return Err(ContractError::InvalidLeverage);
    }

    let price_impact_p = get_price_impact(
        0,
        trade.pair_index,
        trade.long,
        trade.position_size_usd,
    );

    if (price_impact_p * trade.leverage) > state.max_negative_pnl_on_open_p {
        return Err(ContractError::PriceImpactTooHigh);
    }

    if trade.trade_type != TradeType::MARKET {
        let trade_info = state.max_slippage_p;
        store_trade(trade, trade_info)
    } else {
        store_pending_order(trade)
    }

    register_potential_referrer(&state, &info, &trade)?;

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
                    deps.querier.into(),
                    new_trade,
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
    if nft_type < 1 || nft_type > 5 {
        return Err(ContractError::InvalidNftType);
    }

    // Check NFT ownership and timelock
    let nft_owner = deps.api.addr_validate(&trader)?;
    let block_height = env.block.height;
    let state = STATE.load(deps.storage)?;

    if state.is_paused {
        return Err(ContractError::OperationsHalted);
    }

    let nft_last_success =
        state.limit_order_last_success.get(&nft_id).unwrap_or(&0);
    if block_height < *nft_last_success + state.limit_orders_timelock {
        return Err(ContractError::NftSuccessTimelock);
    }

    match order_type {
        LimitOrder::OPEN => {
            let open_limit_order = state
                .open_limit_orders
                .get(&(nft_owner.clone(), pair_index))
                .and_then(|orders| orders.get(&index))
                .ok_or(ContractError::LimitOrderDoesNotExist)?;

            let leveraged_pos_dai = open_limit_order
                .position_size
                .checked_mul(
                    Decimal256::from_atomics(open_limit_order.leverage, 0)
                        .unwrap(),
                )
                .unwrap();

            let price_impact = get_trade_price_impact(
                open_limit_order.pair_index,
                open_limit_order.buy,
                leveraged_pos_dai,
            );

            if price_impact
                .checked_mul(
                    Decimal256::from_atomics(open_limit_order.leverage, 0)
                        .unwrap(),
                )
                .unwrap()
                > state.max_negative_pnl_on_open_p
            {
                return Err(ContractError::PriceImpactTooHigh);
            }

            // Transfer LINK tokens to the price aggregator
            let transfer_msg = BankMsg::Send {
                to_address: state.oracle_address.to_string(),
                amount: vec![Coin {
                    denom: "link".to_string(),
                    amount: Uint128::from(leveraged_pos_dai.to_uint_floor()),
                }],
            };

            // Store pending NFT order
            let order_id = state.next_order_id;
            let pending_nft_order = PendingNftOrder {
                nft_holder: info.sender.clone(),
                trader: nft_owner.clone(),
                pair_index,
                index,
                order_type,
            };

            state.pending_nft_orders.insert(order_id, pending_nft_order);
            state.next_order_id += 1;
            STATE.save(deps.storage, &state)?;

            let response = Response::new()
                .add_message(transfer_msg)
                .add_attribute("action", "execute_limit_order")
                .add_attribute("order_id", order_id.to_string());

            Ok(response)
        }
        _ => Err(ContractError::InvalidLimitOrderType),
    }
}
