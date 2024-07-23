use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Decimal256, Timestamp, Uint128};
use cw_storage_plus::Map;
use std::collections::HashMap;

#[cw_serde]
pub struct Trader {
    pub leverage_unlocked: u64,
    pub referral: Addr,
    pub referral_rewards_total: u128,
}

#[cw_serde]
pub struct Trade {
    pub user: Addr,
    pub pair_index: u64,

    pub leverage: Uint128,
    pub long: bool,
    pub is_open: bool,
    pub collateral_index: u64,
    pub trade_type: TradeType,
    pub collateral_amount: Uint128,

    pub open_price: Decimal,

    pub tp: Decimal,
    pub sl: Decimal,

    pub wanted_price: Decimal,
    pub max_slippage_p: Decimal,
}

#[cw_serde]
pub struct TradeInfo {
    pub created_block: u64,
    pub tp_last_updated_block: u64,
    pub sl_last_updated_block: u64,
    pub last_oi_update_ts: Timestamp,
}

#[cw_serde]
pub enum TradeType {
    Trade,
    Limit,
    Stop,
}

#[cw_serde]
pub enum PendingOrderType {
    MarketOpen,
    MarketClose,
    LimitOpen,
    StopOpen,
    TpClose,
    SlClose,
    LiqClose,
    UpdateLeverage,
    MarketPartialOpen,
    MarketPartialClose,
}

#[cw_serde]
pub struct PendingOrder {
    pub trade: Trade,
    pub user: Addr,
    pub index: u32,
    pub is_open: bool,
    pub order_type: PendingOrderType,
    pub created_block: u32,
    pub max_slippage_p: Decimal256,
}

#[cw_serde]
pub struct OpenLimitOrder {
    pub trader: Addr,
    pub pair_index: u64,
    pub index: u64,
    pub position_size: Decimal256,
    pub spread_reduction_p: u64,
    pub buy: bool,
    pub leverage: u64,
    pub tp: Decimal256,
    pub sl: Decimal256,
    pub min_price: Decimal256,
    pub max_price: Decimal256,
    pub block: u64,
    pub token_id: u64,
}

#[cw_serde]
pub struct PendingMarketOrder {
    pub trade: Trade,
    pub block: u64,
    pub wanted_price: Decimal256,
    pub slippage_p: u64,
    pub spread_reduction_p: Decimal256,
    pub token_id: u64,
}

#[cw_serde]
pub struct PendingNftOrder {
    pub nft_holder: Addr,
    pub trader: Addr,
    pub pair_index: u64,
    pub index: u64,
    pub order_type: LimitOrder,
}

#[cw_serde]
pub struct PairParams {
    pub one_percent_depth_above: u128,
    pub one_percent_depth_below: u128,
    pub rollover_fee_per_block_p: u128,
    pub funding_fee_per_block_p: u128,
}

#[cw_serde]
pub struct PairFundingFees {
    pub acc_per_oi_long: i128,
    pub acc_per_oi_short: i128,
    pub last_update_block: u64,
}

#[cw_serde]
pub struct PairRolloverFees {
    pub acc_per_collateral: u128,
    pub last_update_block: u64,
}

#[cw_serde]
pub struct TradeInitialAccFees {
    pub rollover: u128,
    pub funding: i128,
    pub opened_after_update: bool,
}

#[cw_serde]
pub struct AggregatorAnswer {
    pub order_id: u64,
    pub price: u128,
    pub spread_p: u64,
}

#[cw_serde]
pub struct Values {
    pub price: u128,
    pub profit_p: i128,
    pub token_price_nusd: u128,
    pub pos_token: u128,
    pub pos_nusd: u128,
}

#[cw_serde]
pub enum LimitOrder {
    TP,
    SL,
    LIQ,
    OPEN,
}

#[cw_serde]
pub enum OpenOrderType {
    /// Market orders, order is opened as long as the price is within the
    /// limits of the order.
    MARKET,

    /// Reversal limit order, order is opened when the price goes beyond the
    /// limits of the order in the opposite direction.
    REVERSAL,

    /// Momentum limit order, order is opened when the price goes beyond the
    /// limits of the order in the same direction.
    MOMENTUM,
}

pub const COLLATERALS: Map<String, u64> = Map::new("collaterals");
pub const TRADES: Map<(Addr, u64), Trade> = Map::new("trades");
pub const TRADE_INFOS: Map<(Addr, u64), TradeInfo> = Map::new("trade_infos");
pub const TRADE_PENDING_ORDERS_BLOCK: Map<(Addr, u64, PendingOrder), u64> =
    Map::new("pending_orders");
pub const PENDING_ORDERS: Map<(Addr, u64), PendingOrder> =
    Map::new("pending_orders");
pub const TRADER_STORED: Map<Addr, bool> = Map::new("trader_stored");
