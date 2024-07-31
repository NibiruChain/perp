use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::{
    borrowing::state::{BorrowingData, BorrowingPairGroup, OpenInterest},
    fees::state::{FeeTier, TraderDailyInfo},
    pairs::state::{Fee, Group, Pair},
    price_impact::state::{OiWindowsSettings, PairDepth, PairOi},
    trading::state::{LimitOrder, OpenOrderType, Trade, TradeInfo},
};

#[cw_serde]
pub enum ExecuteMsg {
    /// Opens a new trade with specified parameters.
    /// Parameters:
    /// - trade: Trade details including position size, leverage, and other parameters.
    /// - order_type: The type of order (e.g., legacy, reversal, momentum).
    /// - spread_reduction_id: ID for any spread reduction applicable.
    /// - slippage_p: Slippage percentage for market orders.
    /// - referral: Referral address for tracking referral rewards.
    OpenTrade {
        trade: Trade,
        order_type: OpenOrderType,
        spread_reduction_id: u64,
        slippage_p: Decimal,
        referral: String,
    },

    /// Closes an open trade for the specified pair index and trade index.
    /// Parameters:
    /// - pair_index: The index of the trading pair.
    /// - index: The index of the trade to be closed.
    CloseTradeMarket { pair_index: u64, index: u64 },

    /// Updates the open limit order with new parameters.
    /// Parameters:
    /// - pair_index: The index of the trading pair.
    /// - index: The index of the limit order to update.
    /// - price: New price for the limit order.
    /// - tp: New take profit value.
    /// - sl: New stop loss value.
    UpdateOpenLimitOrder {
        pair_index: u64,
        index: u64,
        price: Decimal,
        tp: Decimal,
        sl: Decimal,
    },

    /// Cancels the open limit order.
    /// Parameters:
    /// - pair_index: The index of the trading pair.
    /// - index: The index of the limit order to cancel.
    CancelOpenLimitOrder { pair_index: u64, index: u64 },

    /// Updates the take profit value for an open trade.
    /// Parameters:
    /// - pair_index: The index of the trading pair.
    /// - index: The index of the trade to update.
    /// - new_tp: The new take profit value.
    UpdateTp {
        pair_index: u64,
        index: u64,
        new_tp: Decimal,
    },

    /// Updates the stop loss value for an open trade.
    /// Parameters:
    /// - pair_index: The index of the trading pair.
    /// - index: The index of the trade to update.
    /// - new_sl: The new stop loss value.
    UpdateSl {
        pair_index: u64,
        index: u64,
        new_sl: Decimal,
    },

    /// Executes an NFT order for the specified parameters.
    /// Parameters:
    /// - order_type: Type of limit order (TP, SL, LIQ, OPEN).
    /// - trader: Address of the trader.
    /// - pair_index: The index of the trading pair.
    /// - index: The index of the trade or order.
    /// - nft_id: ID of the NFT used.
    /// - nft_type: Type of NFT (1-5).
    ExecuteNftOrder {
        order_type: LimitOrder,
        trader: String,
        pair_index: u64,
        index: u64,
        nft_id: u64,
        nft_type: u8,
    },

    /// Admin executes the specified message.
    /// Parameters:
    /// - msg: The admin message to execute.
    AdminMsg { msg: AdminExecuteMsg },
}

#[cw_serde]
pub enum AdminExecuteMsg {
    // Pairs
    SetPairs {
        pairs: HashMap<u64, Pair>,
    },
    SetGroups {
        groups: HashMap<u64, Group>,
    },
    SetFees {
        fees: HashMap<u64, Fee>,
    },
    SetPairCustomMaxLeverage {
        pair_custom_max_leverage: HashMap<u64, Uint128>,
    },
    UpdateOracleAddress {
        oracle_address: String,
    },
    UpdateStakingAddress {
        staking_address: String,
    },

    // Fees
    UpdateFeeTiers {
        fee_tiers: [FeeTier; 8],
    },
    UpdatePendingGovFees {
        pending_gov_fees: HashMap<u64, Uint128>,
    },
    UpdateTraderDailyInfos {
        trader_daily_infos: HashMap<(String, u64), TraderDailyInfo>,
    },

    // Borrowing
    UpdateBorrowingPairs {
        borrowing_pairs: HashMap<(u64, u64), BorrowingData>,
    },
    UpdateBorrowingPairGroups {
        pair_groups: HashMap<(u64, u64), Vec<BorrowingPairGroup>>,
    },
    UpdateBorrowingPairOis {
        pair_ois: HashMap<(u64, u64), OpenInterest>,
    },
    UpdateBorrowingGroups {
        groups: HashMap<(u64, u64), BorrowingData>,
    },
    UpdateBorrowingGroupOis {
        group_ois: HashMap<(u64, u64), OpenInterest>,
    },

    // Price impact
    UpdateOiWindowsSettings {
        oi_windows_settings: OiWindowsSettings,
    },
    UpdateWindows {
        windows: HashMap<(u64, u64, u64), PairOi>,
    },
    UpdatePairDepths {
        pair_depths: HashMap<u64, PairDepth>,
    },

    // Trading
    UpdateCollaterals {
        collaterals: HashMap<u64, String>,
    },
    UpdateTrades {
        trades: HashMap<(Addr, u64), Trade>,
    },
    UpdateTradeInfos {
        trade_infos: HashMap<(Addr, u64), TradeInfo>,
    },
    UpdateTraderStored {
        trader_stored: HashMap<Addr, bool>,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner is the only one that can use ExecuteMsg.
    pub owner: Option<String>,
    pub staking_address: Option<String>,
    pub oracle_address: Option<String>,
}

#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    /// HasOpenLimitOrder returns whether the trader has an open limit order
    #[returns(bool)]
    HasOpenLimitOrder {
        address: String,
        pair_index: u64,
        index: u64,
    },
}
