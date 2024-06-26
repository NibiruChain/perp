use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};

use crate::msgs::QueryMsg;

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
        } => {
            // todo!();
            Ok(Response::default())
        }
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
