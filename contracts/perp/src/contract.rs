use cosmwasm_std::{
    Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::{
    msgs::QueryMsg,
    state::{LimitOrder, OpenLimitOrderType, Trade, STATE},
};

use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msgs::{ExecuteMsg, InstantiateMsg},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(
        deps.storage,
        format!("crates.io:{CONTRACT_NAME}"),
        CONTRACT_VERSION,
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract_addr = env.contract.address.to_string();
    match msg {
        ExecuteMsg::OpenTrade {
            trade,
            order_type,
            spread_reduction_id,
            slippage_p,
            referral,
        } => open_trade(
            deps,
            env,
            info,
            trade,
            order_type,
            spread_reduction_id,
            slippage_p,
        ),
        ExecuteMsg::CloseTradeMarket { pair_index, index } => {
            // todo!();
            Ok(Response::default())
        }
        ExecuteMsg::UpdateOpenLimitOrder {
            pair_index,
            index,
            price,
            tp,
            sl,
        } => {
            // todo!();
            Ok(Response::default())
        }
        ExecuteMsg::CancelOpenLimitOrder { pair_index, index } => {
            // todo!();
            Ok(Response::default())
        }
        ExecuteMsg::UpdateTp {
            pair_index,
            index,
            new_tp,
        } => {
            // todo!();
            Ok(Response::default())
        }
        ExecuteMsg::UpdateSl {
            pair_index,
            index,
            new_sl,
        } => {
            // todo!();
            Ok(Response::default())
        }
        ExecuteMsg::ExecuteNftOrder {
            order_type,
            trader,
            pair_index,
            index,
            nft_id,
            nft_type,
        } => {
            // todo!();
            Ok(Response::default())
        }
        ExecuteMsg::AdminMsg { msg } => {
            // todo!();
            Ok(Response::default())
        }
    }
}

fn open_trade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    trade: Trade,
    order_type: OpenLimitOrderType,
    spread_reduction_id: u64,
    slippage_p: u64,
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

    // check trade count
    let key = (info.sender, trade.pair_index);

    if state.open_trades_count.get(&key).unwrap_or(&0)
        + state.pending_market_open_count.get(&key).unwrap_or(&0)
        + state.pending_market_close_count.get(&key).unwrap_or(&0)
        >= state.max_trades_per_pair
    {
        return Err(ContractError::MaxTradesPerPair);
    }

    if state
        .pending_order_ids
        .get(&info.sender)
        .unwrap_or(&vec![])
        .len()
        >= state.max_pending_market_orders as usize
    {
        return Err(ContractError::MaxPendingOrders);
    }

    if trade.position_size_nusd > state.max_position_size_nusd {
        return Err(ContractError::InvalidPositionSize);
    }

    if trade
        .position_size_nusd
        .checked_mul(Decimal256::from(trade.leverage.into()))
        .unwrap()
        < *state
            .min_lev_pos
            .get(&trade.pair_index)
            .unwrap_or(&Decimal256::zero())
    {
        return Err(ContractError::InvalidLeverage);
    }

    if trade.leverage < 0
        || trade.leverage
            >= *state.min_leverage.get(&trade.pair_index).unwrap_or(&0)
        || trade.leverage
            <= *state.max_leverage.get(&trade.pair_index).unwrap_or(&0)
    {
        return Err(ContractError::InvalidLeverage);
    }
    if trade.tp != 0
        && ((trade.buy && trade.tp <= trade.open_price)
            || (!trade.buy && trade.tp >= trade.open_price))
    {
        return Err(ContractError::InvalidTpSl);
    }

    if trade.sl != 0
        && ((trade.buy && trade.sl >= trade.open_price)
            || (!trade.buy && trade.sl <= trade.open_price))
    {
        return Err(ContractError::InvalidTpSl);
    }

    let price_impact = get_trade_price_impact(
        trade.pair_index,
        trade.buy,
        trade.position_size_nusd * trade.leverage,
    );

    if price_impact * leverage >= state.max_negative_pnl_on_open_p {
        return Err(ContractError::PriceImpactTooHigh);
    }

    if order_type != OpenLimitOrderType::LEGACY {
        let index = first_empty_open_limit_index(
            &state.open_limit_orders,
            &info.sender,
            trade.pair_index,
        );

        // udpate state
        let open_limit_order = first_empty_limit_index(
            &state.open_limit_orders,
            &info.sender,
            trade.pair_index,
        );
        state.open_limit_orders.push(open_limit_order);
        state.open_limit_orders_id.insert(
            (info.sender.clone(), trade.pair_index, open_limit_order),
            state.open_limit_orders.len() - 1 as u64,
        );
        state.open_limit_orders_count.insert(
            (info.sender.clone(), trade.pair_index),
            state
                .open_limit_orders_count
                .get(&(info.sender.clone(), trade.pair_index))
                .unwrap_or(&0)
                + 1,
        );
    } else {
        let order_id = get_price(
            trade.pair_index,
            trade.position_size_nusd * trade.leverage,
        );

        // update state
        state.pending_order_ids.insert(
            info.sender.clone(),
            state
                .pending_order_ids
                .get(&info.sender)
                .unwrap_or(&vec![])
                .push(order_id),
        );

        state.pending_market_orders.insert(
            order_id,
            PendingMarketOrder {
                order_id,
                trader: info.sender.clone(),
                pair_index: trade.pair_index,
                position_size_nusd: trade.position_size_nusd,
                leverage: trade.leverage,
                open_price: trade.open_price,
                buy: trade.buy,
                tp: trade.tp,
                sl: trade.sl,
                slippage_p,
                spread_reduction,
                referral,
                timelock: env.block.height + state.market_order_timelock,
            },
        );

        state.pending_market_open_count.insert(
            (info.sender.clone(), trade.pair_index),
            state
                .pending_market_open_count
                .get(&(info.sender.clone(), trade.pair_index))
                .unwrap_or(&0)
                + 1,
        );
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::HasOpenLimitOrder {
            address: String,
            pair_index,
            index: u64,
        } => {
            todo!()
        }
        QueryMsg::OpenTrades {
            address: String,
            pair_index,
            index: u64,
        } => {
            todo!()
        }
        QueryMsg::OpenTradesInfo {
            address: String,
            pair_index,
            index: u64,
        } => {
            todo!()
        }
        QueryMsg::GetOpenLimitOrder {
            address: String,
            pair_index,
            index: u64,
        } => {
            todo!()
        }
        QueryMsg::SpreadReductionsP { id: u64 } => {
            todo!()
        }
        QueryMsg::MaxSlP {} => {
            todo!()
        }
        QueryMsg::ReqIDPendingMarketOrder { order_id: u64 } => {
            todo!()
        }
        QueryMsg::FirstEmptyTradeIndex {
            address: String,
            pair_index: u64,
        } => {
            todo!()
        }
        QueryMsg::FirstEmptyOpenLimitIndex {
            address: String,
            pair_index: u64,
        } => {
            todo!()
        }
        QueryMsg::NftSuccessTimelock {} => {
            todo!()
        }
        QueryMsg::CurrentPercentProfit {
            open_price,
            current_price,
            leverage: u64,
            buy: bool,
        } => {
            todo!()
        }
        QueryMsg::ReqIDPendingNftOrder { order_id: u64 } => {
            todo!()
        }
        QueryMsg::NftLastSuccess { nft_id: u64 } => {
            todo!()
        }
        QueryMsg::GetReferral { address: String } => {
            todo!()
        }
        QueryMsg::GetLeverageUnlocked { address: String } => {
            todo!()
        }
        QueryMsg::OpenLimitOrdersCount {
            address: String,
            pair_index: u64,
        } => {
            todo!()
        }
        QueryMsg::MaxOpenLimitOrdersPerPair {} => {
            todo!()
        }
        QueryMsg::OpenTradesCount {
            address: String,
            pair_index: u64,
        } => {
            todo!()
        }
        QueryMsg::PendingMarketOpenCount {
            address: String,
            pair_index: u64,
        } => {
            todo!()
        }
        QueryMsg::PendingMarketCloseCount {
            address: String,
            pair_index: u64,
        } => {
            todo!()
        }
        QueryMsg::MaxTradesPerPair {} => {
            todo!()
        }
        QueryMsg::MaxTradesPerBlock {} => {
            todo!()
        }
        QueryMsg::TradesPerBlock { pair_index: u64 } => {
            todo!()
        }
        QueryMsg::PendingOrderIdsCount { address: String } => {
            todo!()
        }
        QueryMsg::MaxPendingMarketOrders {} => {
            todo!()
        }
        QueryMsg::MaxGainP {} => {
            todo!()
        }
        QueryMsg::DefaultLeverageUnlocked {} => {
            todo!()
        }
        QueryMsg::OpenInterestDai {
            pair_index,
            index: u64,
        } => {
            todo!()
        }
        QueryMsg::GetPendingOrderIds { address: String } => {
            todo!()
        }
        QueryMsg::Traders { address: String } => {
            todo!()
        }
    }
}
