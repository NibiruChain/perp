use cosmwasm_std::{Binary, Deps, Env};

use crate::{error::ContractError, msgs::QueryMsg};

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
