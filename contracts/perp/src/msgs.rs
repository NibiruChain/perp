use std::collections::BTreeSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;

use crate::trading::state::{
    LimitOrder, OpenLimitOrder, OpenOrderType, PendingMarketOrder,
    PendingNftOrder, Trade, TradeInfo, Trader,
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
    /// Handle tokens by either minting or burning the specified amount.
    /// Parameters:
    /// - address: The address to handle tokens for.
    /// - amount: The amount of tokens to handle.
    /// - mint: A boolean indicating whether to mint (true) or burn (false) tokens.
    HandleTokens {
        address: String,
        amount: u64,
        mint: bool,
    },

    /// Transfer a specified amount of DAI from one address to another.
    /// Parameters:
    /// - from: The address to transfer DAI from.
    /// - to: The address to transfer DAI to.
    /// - amount: The amount of DAI to transfer.
    TransferDai {
        from: String,
        to: String,
        amount: u64,
    },

    /// Transfer LINK tokens to the price aggregator.
    /// Parameters:
    /// - from: The address to transfer LINK from.
    /// - pair_index: The pair index for which the transfer is being made.
    /// - leveraged_pos_dai: The leveraged position in DAI.
    TransferLinkToAggregator {
        from: String,
        pair_index: u64,
        leveraged_pos_dai: u64,
    },

    /// Unregister a trade.
    /// Parameters:
    /// - trader: The address of the trader.
    /// - pair_index: The pair index of the trade.
    /// - index: The index of the trade.
    UnregisterTrade {
        trader: String,
        pair_index: u64,
        index: u64,
    },

    /// Unregister a pending market order.
    /// Parameters:
    /// - order_id: The ID of the pending market order.
    /// - is_open: A boolean indicating whether the order is an open order.
    UnregisterPendingMarketOrder { order_id: u64, is_open: bool },

    /// Unregister an open limit order.
    /// Parameters:
    /// - trader: The address of the trader.
    /// - pair_index: The pair index of the order.
    /// - index: The index of the order.
    UnregisterOpenLimitOrder {
        trader: String,
        pair_index: u64,
        index: u64,
    },

    /// Store a pending market order.
    /// Parameters:
    /// - order: The pending market order to store.
    /// - order_id: The ID of the order.
    /// - is_open: A boolean indicating whether the order is an open order.
    StorePendingMarketOrder {
        order: PendingMarketOrder,
        order_id: u64,
        is_open: bool,
    },

    /// Store a referral.
    /// Parameters:
    /// - trader: The address of the trader.
    /// - referral: The address of the referral.
    StoreReferral { trader: String, referral: String },

    /// Update the stop-loss for a trade.
    /// Parameters:
    /// - trader: The address of the trader.
    /// - pair_index: The pair index of the trade.
    /// - index: The index of the trade.
    /// - new_sl: The new stop-loss value.
    UpdateSl {
        trader: String,
        pair_index: u64,
        index: u64,
        new_sl: u64,
    },

    /// Update the take-profit for a trade.
    /// Parameters:
    /// - trader: The address of the trader.
    /// - pair_index: The pair index of the trade.
    /// - index: The index of the trade.
    /// - new_tp: The new take-profit value.
    UpdateTp {
        trader: String,
        pair_index: u64,
        index: u64,
        new_tp: u64,
    },

    /// Store an open limit order.
    /// Parameters:
    /// - order: The open limit order to store.
    StoreOpenLimitOrder { order: OpenLimitOrder },

    /// Store a pending NFT order.
    /// Parameters:
    /// - order: The pending NFT order to store.
    /// - order_id: The ID of the order.
    StorePendingNftOrder {
        order: PendingNftOrder,
        order_id: u64,
    },

    /// Update an open limit order.
    /// Parameters:
    /// - order: The open limit order to update.
    UpdateOpenLimitOrder { order: OpenLimitOrder },

    /// Increase NFT rewards.
    /// Parameters:
    /// - nft_id: The ID of the NFT.
    /// - amount: The amount of rewards to increase.
    IncreaseNftRewards { nft_id: u64, amount: u64 },

    /// Set the last success block for an NFT.
    /// Parameters:
    /// - nft_id: The ID of the NFT.
    SetNftLastSuccess { nft_id: u64 },

    /// Update a trade.
    /// Parameters:
    /// - trade: The trade to update.
    UpdateTrade { trade: Trade },

    /// Unregister a pending NFT order.
    /// Parameters:
    /// - order_id: The ID of the pending NFT order.
    UnregisterPendingNftOrder { order_id: u64 },

    /// Distribute LP rewards.
    /// Parameters:
    /// - amount: The amount of rewards to distribute.
    DistributeLpRewards { amount: u64 },

    /// Increase referral rewards.
    /// Parameters:
    /// - referral: The address of the referral.
    /// - amount: The amount of rewards to increase.
    IncreaseReferralRewards { referral: String, amount: u64 },

    /// Store a trade.
    /// Parameters:
    /// - trade: The trade to store.
    /// - trade_info: The trade information to store.
    StoreTrade { trade: Trade, trade_info: TradeInfo },

    /// Set the leverage unlocked for a trader.
    /// Parameters:
    /// - trader: The address of the trader.
    /// - leverage: The new leverage value.
    SetLeverageUnlocked { trader: String, leverage: u64 },
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner is the only one that can use ExecuteMsg.
    pub owner: Option<String>,
    pub to_addrs: BTreeSet<String>,
    pub opers: BTreeSet<String>,
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

    /// OpenTrades returns the trade details
    #[returns(Trade)]
    OpenTrades {
        address: String,
        pair_index: u64,
        index: u64,
    },

    /// OpenTradesInfo returns additional information about the open trade
    #[returns(TradeInfo)]
    OpenTradesInfo {
        address: String,
        pair_index: u64,
        index: u64,
    },

    /// GetOpenLimitOrder returns the details of an open limit order
    #[returns(OpenLimitOrder)]
    GetOpenLimitOrder {
        address: String,
        pair_index: u64,
        index: u64,
    },

    /// SpreadReductionsP returns the spread reduction percentage for a given ID
    #[returns(u64)]
    SpreadReductionsP { id: u64 },

    /// MaxSlP returns the maximum stop loss percentage
    #[returns(u64)]
    MaxSlP {},

    /// ReqIDPendingMarketOrder returns the pending market order for a given order ID
    #[returns(PendingMarketOrder)]
    ReqIDPendingMarketOrder { order_id: u64 },

    /// FirstEmptyTradeIndex returns the first empty trade index for the given
    /// address and pair index
    #[returns(u64)]
    FirstEmptyTradeIndex { address: String, pair_index: u64 },

    /// FirstEmptyOpenLimitIndex returns the first empty open limit order index
    /// for the given address and pair index
    #[returns(u64)]
    FirstEmptyOpenLimitIndex { address: String, pair_index: u64 },

    /// NftSuccessTimelock returns the timelock for NFT success
    #[returns(u64)]
    NftSuccessTimelock {},

    /// CurrentPercentProfit returns the current percent profit for the given
    /// trade parameters
    #[returns(Decimal)]
    CurrentPercentProfit {
        open_price: u64,
        current_price: u64,
        buy: bool,
        leverage: u64,
    },

    /// ReqIDPendingNftOrder returns the pending NFT order for a given order ID
    #[returns(PendingNftOrder)]
    ReqIDPendingNftOrder { order_id: u64 },

    /// NftLastSuccess returns the timestamp of the last successful NFT order
    /// for the given NFT ID
    #[returns(u64)]
    NftLastSuccess { nft_id: u64 },

    /// GetReferral returns the referral address for the given trader
    #[returns(String)]
    GetReferral { address: String },

    /// GetLeverageUnlocked returns the leverage unlocked for the given trader
    #[returns(u64)]
    GetLeverageUnlocked { address: String },

    /// OpenLimitOrdersCount returns the count of open limit orders for the
    /// given trader and pair index
    #[returns(u64)]
    OpenLimitOrdersCount { address: String, pair_index: u64 },

    /// MaxOpenLimitOrdersPerPair returns the maximum number of open limit
    /// orders allowed per pair
    #[returns(u64)]
    MaxOpenLimitOrdersPerPair {},

    /// OpenTradesCount returns the count of open trades for the given trader
    /// and pair index
    #[returns(u64)]
    OpenTradesCount { address: String, pair_index: u64 },

    /// PendingMarketOpenCount returns the count of pending market open orders
    /// for the given trader and pair index
    #[returns(u64)]
    PendingMarketOpenCount { address: String, pair_index: u64 },

    /// PendingMarketCloseCount returns the count of pending market close
    /// orders for the given trader and pair index
    #[returns(u64)]
    PendingMarketCloseCount { address: String, pair_index: u64 },

    /// MaxTradesPerPair returns the maximum number of trades allowed per pair
    #[returns(u64)]
    MaxTradesPerPair {},

    /// MaxTradesPerBlock returns the maximum number of trades allowed per block
    #[returns(u64)]
    MaxTradesPerBlock {},

    /// TradesPerBlock returns the count of trades in the current block for
    /// the given pair index
    #[returns(u64)]
    TradesPerBlock { pair_index: u64 },

    /// PendingOrderIdsCount returns the count of pending order IDs for the
    /// given trader
    #[returns(u64)]
    PendingOrderIdsCount { address: String },

    /// MaxPendingMarketOrders returns the maximum number of pending market
    /// orders allowed
    #[returns(u64)]
    MaxPendingMarketOrders {},

    /// MaxGainP returns the maximum gain percentage allowed
    #[returns(u64)]
    MaxGainP {},

    /// DefaultLeverageUnlocked returns the default leverage unlocked for
    /// traders
    #[returns(u64)]
    DefaultLeverageUnlocked {},

    /// OpenInterestDai returns the open interest in DAI for the given pair
    /// index and trade index
    #[returns(u64)]
    OpenInterestDai { pair_index: u64, index: u64 },

    /// GetPendingOrderIds returns a list of pending order IDs for the given
    /// trader
    #[returns(Vec<u64>)]
    GetPendingOrderIds { address: String },

    /// Traders returns the trader details for the given address
    #[returns(Trader)]
    Traders { address: String },
}
