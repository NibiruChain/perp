use crate::borrowing::{
    get_trade_liquidation_price_with_fees, handle_trade_borrowing,
};
use crate::constants::{MAX_OPEN_NEGATIVE_PNL_P};
use crate::error::ContractError;
use crate::fees::{process_closing_fees, process_opening_fees};
use crate::pairs::state::{
    FEES, GROUPS, ORACLE_ADDRESS, PAIRS, PAIR_CUSTOM_MAX_LEVERAGE,
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
use crate::trading::utils::{
    get_collateral_price, get_collateral_price_usd, get_pnl_percent,
    get_position_size_collateral, limit_sl_distance, limit_tp_distance,
};
use crate::utils::{u128_to_dec};
use cosmwasm_std::{
    Addr, BankMsg, BlockInfo, Coin, Decimal, Deps, DepsMut, Int128, MessageInfo,
    Response, SignedDecimal, Uint128,
};

use oracle::contract::OracleQueryMsg;

pub fn open_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
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

    if trade.trade_type != TradeType::Trade {
        // limit orders are stored as such in the same state, we just don't
        // update the open interest since they are not "live"
        return store_trade(
            deps,
            block,
            trade.clone(),
            None,
            Some(max_slippage_p),
        );
    } else {
        trade.validate(
            deps.as_ref(),
            block,
            position_size_usd,
            base_price,
            base_price,
            max_slippage_p,
        )?;
        let height = block.height;
        let time = block.time;

        let trade_info = TradeInfo {
            created_block: height,
            tp_last_updated_block: height,
            sl_last_updated_block: height,
            last_oi_update_ts: time,
            max_slippage_p,
            collateral_price_usd: collateral_price,
        };

        return register_trade(
            deps,
            block,
            trade.clone(),
            trade_info,
            order_type,
        );
    }
    register_potential_referrer(&trade)?;
}

// Validate the trade and store it as a trade
fn register_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trade: Trade,
    trade_info: TradeInfo,
    order_type: OpenOrderType,
) -> Result<Response, ContractError> {
    let mut final_trade = trade.clone();
    let (msgs, fees) = process_opening_fees(
        deps,
        block,
        trade.clone(),
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
        order_type,
    )?;
    final_trade.collateral_amount -= fees;
    store_trade(deps, block, final_trade, Some(trade_info), None)?;

    Ok(Response::new().add_messages(msgs))
}

pub fn get_token_price(
    deps: &Deps,
    oracle_index: &u64,
) -> Result<Decimal, ContractError> {
    Ok(deps.querier.query_wasm_smart::<Decimal>(
        ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        &OracleQueryMsg::GetPrice {
            index: *oracle_index,
        },
    )?)
}

fn register_potential_referrer(_trade: &Trade) -> Result<(), ContractError> {
    todo!()
}

fn store_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trade: Trade,
    trade_info: Option<TradeInfo>,
    max_slippage_p: Option<Decimal>,
) -> Result<Response, ContractError> {
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
                created_block: block.height,
                tp_last_updated_block: block.height,
                sl_last_updated_block: block.height,
                last_oi_update_ts: block.time,
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
        add_trade_oi_collateral(block, deps, trade, trade_info)?;
    }
    Ok(Response::new().add_attribute("action", "open_trade"))
}

fn add_trade_oi_collateral(
    block: &BlockInfo,
    deps: &mut DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
) -> Result<(), ContractError> {
    add_oi_collateral(
        block,
        deps,
        trade.clone(),
        trade_info,
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
    )
}

fn add_oi_collateral(
    block: &BlockInfo,
    deps: &mut DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    handle_trade_borrowing(
        block,
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
        block,
        trade,
        trade_info,
        position_collateral,
    )?;
    Ok(())
}

fn remove_trade_oi_collateral(
    block: &BlockInfo,
    deps: &mut DepsMut,
    trade: Trade,
) -> Result<(), ContractError> {
    remove_oi_collateral(
        deps,
        block,
        trade.clone(),
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?,
    )
}

fn remove_oi_collateral(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trade: Trade,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    handle_trade_borrowing(
        block,
        trade.user.clone(),
        trade.index,
        deps.storage,
        trade.collateral_index,
        trade.pair_index,
        position_collateral,
        false,
        trade.long,
    )?;
    remove_price_impact_open_interest(deps, block, trade, position_collateral)?;

    Ok(())
}

fn get_usd_normalized_value(
    collateral_price: Decimal,
    collateral_value: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(collateral_price
        .checked_mul(u128_to_dec(collateral_value)?)?
        .to_uint_floor())
}

/// used to close open limit orders, not trade market
pub fn close_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
    info: MessageInfo,
    index: u64,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_messages(_close_trade(deps, block, info.sender, index)?)
        .add_attribute("action", "cance_open_order"))
}

fn _close_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trader: Addr,
    index: u64,
) -> Result<Vec<BankMsg>, ContractError> {
    let mut trade = TRADES.load(deps.storage, (trader.clone(), index))?;

    if !trade.is_open {
        return Err(ContractError::TradeClosed);
    }

    trade.is_open = false;
    let counter = USER_COUNTERS.load(deps.storage, trader.clone())?;
    USER_COUNTERS.save(deps.storage, trader.clone(), &(counter - 1))?;
    TRADES.save(deps.storage, (trader.clone(), index), &trade)?;

    if trade.trade_type == TradeType::Trade {
        remove_trade_oi_collateral(block, deps, trade)?;
        Ok(vec![])
    } else {
        Ok(vec![BankMsg::Send {
            to_address: trader.to_string(),
            amount: vec![Coin::new(
                trade.collateral_amount,
                COLLATERALS.load(deps.storage, trade.collateral_index)?,
            )],
        }])
    }
}

pub fn update_open_order(
    deps: &mut DepsMut,
    block: &BlockInfo,
    info: MessageInfo,
    index: u64,
    price: Decimal,
    tp: Decimal,
    sl: Decimal,
    slippage_p: Decimal,
) -> Result<Response, ContractError> {
    let trade = TRADES.load(deps.storage, (info.sender.clone(), index))?;
    update_open_order_details(deps, block, trade, price, tp, sl, slippage_p)?;
    Ok(Response::new().add_attribute("action", "update_open_order"))
}

fn update_open_order_details(
    deps: &mut DepsMut,
    block: &BlockInfo,
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
    trade_info.created_block = block.height;
    trade_info.tp_last_updated_block = block.height;
    trade_info.sl_last_updated_block = block.height;

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
    block: &BlockInfo,
    info: MessageInfo,
    index: u64,
) -> Result<Response, ContractError> {
    close_trade(deps, block, info, index)
}

pub fn update_tp(
    deps: &mut DepsMut,
    block: &BlockInfo,
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
    trade_info.tp_last_updated_block = block.height;

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
    block: &BlockInfo,
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
    trade_info.sl_last_updated_block = block.height;

    TRADES.save(deps.storage, (trade.user.clone(), trade.pair_index), &trade)?;
    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.pair_index),
        &trade_info,
    )?;
    Ok(Response::new().add_attribute("action", "update_sl"))
}

pub fn trigger_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trader: Addr,
    info: MessageInfo,
    index: u64,
    mut pending_order_type: PendingOrderType,
) -> Result<Response, ContractError> {
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
            block,
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

    let _position_size_collateral =
        get_position_size_collateral(trade.collateral_amount, trade.leverage)?;

    if is_open_limit {
        let leveraged_pos_usd = get_position_size_collateral(
            trade.collateral_amount,
            trade.leverage,
        )?;

        let (price_impact_p, _) = get_trade_price_impact(
            deps.storage,
            block,
            Decimal::zero(),
            trade.pair_index,
            trade.long,
            leveraged_pos_usd,
        )?;

        if price_impact_p.checked_mul(u128_to_dec(trade.leverage)?)?
            > MAX_OPEN_NEGATIVE_PNL_P
        {
            return Err(ContractError::PriceImpactTooHigh);
        }
    }

    let trigger_price = get_token_price(&deps.as_ref(), &trade.pair_index)?;
    if trigger_price.is_zero() {
        return Err(ContractError::TradeInvalid);
    }

    match pending_order_type {
        PendingOrderType::LimitOpen | PendingOrderType::StopOpen => {
            trigger_open_order(
                deps,
                block,
                info,
                trade,
                trigger_price,
                pending_order_type,
            )
        }
        PendingOrderType::TpClose
        | PendingOrderType::SlClose
        | PendingOrderType::LiqClose => trigger_close_order(
            deps,
            block,
            info,
            trade,
            trigger_price,
            pending_order_type,
        ),
        PendingOrderType::Market => Err(ContractError::InvalidTradeType),
    }
}

fn trigger_close_order(
    deps: &mut DepsMut,
    block: &BlockInfo,
    _info: MessageInfo,
    trade: Trade,
    price: Decimal,
    pending_order_type: PendingOrderType,
) -> Result<Response, ContractError> {
    let trigger_price = match pending_order_type {
        PendingOrderType::TpClose => trade.tp,
        PendingOrderType::SlClose => trade.sl,
        PendingOrderType::LiqClose => get_trade_liquidation_price_with_fees(
            &deps.as_ref(),
            block,
            trade.clone(),
            true,
        )?,
        _ => return Err(ContractError::InvalidTradeType),
    };

    if is_hit(trade.long, pending_order_type.clone(), price, trigger_price) {
        let profit_p = get_pnl_percent(
            trade.open_price,
            price,
            trade.long,
            trade.leverage,
        )?;

        let response = unregister_trade(
            deps,
            block,
            trade.clone(),
            profit_p,
            pending_order_type,
        )?;

        Ok(response.add_attribute("action", "close_trade"))
    } else {
        Err(ContractError::InvalidTriggerPrice)
    }
}

fn unregister_trade(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trade: Trade,
    profit_p: SignedDecimal,
    pending_order_type: PendingOrderType,
) -> Result<Response, ContractError> {
    let (
        mut msgs,
        vault_closing_fee_collateral,
        _gov_staking_fee_collateral,
        trigger_fee_collateral,
        collateral_left_in_storage,
    ) = process_closing_fees(
        deps,
        block,
        trade.clone(),
        trade.get_position_size_collateral(),
        pending_order_type.clone(),
    )?;

    let (trade_value_collateral, borrowing_fee_collateral) = trade
        .get_trade_value_collateral(
            &deps.as_ref(),
            block,
            profit_p,
            vault_closing_fee_collateral + trigger_fee_collateral,
            pending_order_type,
        )?;

    let (_bad_debt, pnl_message) = handle_trade_pnl(
        COLLATERALS.load(deps.storage, trade.collateral_index)?,
        trade.clone(),
        Int128::try_from(trade_value_collateral).unwrap(),
        Int128::try_from(collateral_left_in_storage).unwrap(),
        borrowing_fee_collateral,
    )?;

    if let Some(message) = pnl_message {
        msgs.push(message);
    }

    let resp = _close_trade(deps, block, trade.user.clone(), trade.index)?;

    msgs.extend(resp);
    Ok(Response::new().add_messages(msgs))
}

/// Handles PnL (Profit and Loss) transfers when (fully or partially) closing a trade.
///
/// This function processes the collateral that needs to be sent to the trader when a trade is closed. It ensures
/// that the appropriate amount of collateral is sent, considering the available collateral and borrowing fees.
/// If the available collateral is insufficient, it records the debt the trader owes.
///
/// # Arguments
///
/// * `deps` - A reference to the dependencies required for executing the function. This includes access to storage, API, etc.
/// * `trade` - The trade struct containing details of the trade being closed.
/// * `collateral_sent_to_trader` - The total amount of collateral to send to the trader (in collateral precision).
/// * `available_collateral` - The part of `collateral_sent_to_trader` that is available in the system's balance (in collateral precision).
/// * `borrowing_fee_collateral` - The collateral amount representing the borrowing fee.
fn handle_trade_pnl(
    collateral_denom: String,
    trade: Trade,
    collateral_sent_to_trader: Int128,
    available_collateral: Int128,
    _borrowing_fee_collateral: Uint128,
) -> Result<(Uint128, Option<BankMsg>), ContractError> {
    let mut trader_debt = Uint128::zero();
    let mut message: Option<BankMsg> = None;

    if collateral_sent_to_trader > available_collateral {
        if !available_collateral.is_negative() {
            message = Some(BankMsg::Send {
                to_address: trade.user.to_string(),
                amount: vec![Coin::new(
                    Uint128::try_from(available_collateral).unwrap(),
                    collateral_denom.clone(),
                )],
            });
        } else {
            trader_debt = available_collateral.unsigned_abs();
        }
    } else if !collateral_sent_to_trader.is_negative() {
        message = Some(BankMsg::Send {
            to_address: trade.user.to_string(),
            amount: vec![Coin::new(
                Uint128::try_from(collateral_sent_to_trader).unwrap(),
                collateral_denom,
            )],
        });
    } else {
        trader_debt = collateral_sent_to_trader.unsigned_abs();
    }
    Ok((trader_debt, message))
}

fn is_hit(
    long: bool,
    pending_order_type: PendingOrderType,
    price: Decimal,
    trigger_price: Decimal,
) -> bool {
    (pending_order_type == PendingOrderType::TpClose
        && ((long && price >= trigger_price)
            || (!long && price <= trigger_price)))
        || (pending_order_type == PendingOrderType::SlClose
            && ((long && price <= trigger_price)
                || (!long && price >= trigger_price)))
        || (pending_order_type == PendingOrderType::LiqClose
            && ((long && price <= trigger_price)
                || (!long && price >= trigger_price)))
}

fn trigger_open_order(
    deps: &mut DepsMut,
    block: &BlockInfo,
    info: MessageInfo,
    trade: Trade,
    trigger_price: Decimal,
    _pending_order_type: PendingOrderType,
) -> Result<Response, ContractError> {
    let mut trade = trade.clone();
    let trade_info =
        TRADE_INFOS.load(deps.storage, (trade.user.clone(), trade.index))?;

    let (_, price_after_impact) = trade.validate(
        deps.as_ref(),
        block,
        get_usd_normalized_value(
            get_collateral_price_usd(&deps.as_ref(), trade.collateral_index)?,
            trade.get_position_size_collateral(),
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

    close_trade(deps, block, info, trade.index)?;

    // register the market trade
    trade.open_price = price_after_impact;
    trade.trade_type = TradeType::Trade;

    register_trade(deps, block, trade, trade_info, OpenOrderType::MARKET)
}
