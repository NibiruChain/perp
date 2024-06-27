use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256};
use cw_storage_plus::Item;
use std::collections::HashMap;

#[cw_serde]
pub struct Trader {
    pub leverage_unlocked: u64,
    pub referral: Addr,
    pub referral_rewards_total: u128,
}

#[cw_serde]
pub struct Trade {
    pub trader: Addr,
    pub pair_index: u64,
    pub index: u64,
    pub initial_pos_token: u128,
    pub position_size_nusd: Decimal256,
    pub open_price: u128,
    pub buy: bool,
    pub leverage: u64,
    pub tp: u128,
    pub sl: u128,
}

#[cw_serde]
pub struct TradeInfo {
    pub token_id: u64,
    pub token_price_nusd: u128,
    pub open_interest_nusd: u128,
    pub tp_last_updated: u64,
    pub sl_last_updated: u64,
    pub being_market_closed: bool,
}

#[cw_serde]
pub struct OpenLimitOrder {
    pub trader: Addr,
    pub pair_index: u64,
    pub index: u64,
    pub position_size: u128,
    pub spread_reduction_p: u64,
    pub buy: bool,
    pub leverage: u64,
    pub tp: u128,
    pub sl: u128,
    pub min_price: u128,
    pub max_price: u128,
    pub block: u64,
    pub token_id: u64,
}

#[cw_serde]
pub struct PendingMarketOrder {
    pub trade: Trade,
    pub block: u64,
    pub wanted_price: u128,
    pub slippage_p: u64,
    pub spread_reduction_p: u64,
    pub token_id: u64,
}

#[cw_serde]
pub struct PendingNftOrder {
    pub nft_holder: Addr,
    pub nft_id: u64,
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
    pub nft_reward: u128,
}

#[cw_serde]
pub enum LimitOrder {
    TP,
    SL,
    LIQ,
    OPEN,
}

#[cw_serde]
pub enum OpenLimitOrderType {
    LEGACY,
    REVERSAL,
    MOMENTUM,
}

pub const STATE: Item<State> = Item::new("state");

#[cw_serde]
pub struct State {
    // user info mapping
    pub traders: HashMap<Addr, Trader>,

    // trade mappings
    pub open_trades: HashMap<(Addr, u64, u64), Trade>,
    pub open_trades_info: HashMap<(Addr, u64, u64), TradeInfo>,
    pub open_trades_count: HashMap<(Addr, u64), u64>,

    // limit orders mappings
    pub open_limit_orders: Vec<OpenLimitOrder>,
    pub open_limit_order_ids: HashMap<(Addr, u64, u64), u64>,
    pub open_limit_orders_count: HashMap<(Addr, u64), u64>,

    // Pending orders mappings
    pub pending_market_orders: HashMap<u64, PendingMarketOrder>,
    pub pending_order_ids: HashMap<Addr, Vec<u64>>,
    pub pending_market_open_count: HashMap<(Addr, u64), u64>,
    pub pending_market_close_count: HashMap<(Addr, u64), u64>,

    // list of open trades & limit orders
    pub pair_traders: HashMap<u64, Vec<Addr>>,
    pub pair_traders_id: HashMap<(Addr, u64), u64>,

    // Current and max open interests for each pair
    pub open_interest: HashMap<u64, [Decimal256; 3]>, // [long, short, max]

    // Restrictions & Timelocks
    pub trades_per_block: HashMap<u64, u64>,

    // pair infos
    pub max_negative_pnl_on_open_p: Decimal256,
    pub pair_params: HashMap<u64, PairParams>,
    pub pair_funding_fees: HashMap<u64, PairFundingFees>,
    pub pair_rollover_fees: HashMap<u64, PairRolloverFees>,
    pub trade_initial_acc_fees: HashMap<(Addr, u64, u64), TradeInitialAccFees>,

    // trading variables
    pub max_trades_per_pair: u64,
    pub max_trade_per_block: u64,
    pub max_pending_market_orders: u64,
    pub max_gain_p: Decimal256,
    pub max_sl_p: Decimal256,
    pub default_leverage_unlocked: u64,
    pub spread_reductions_p: [Decimal256; 5],

    // trading callbacks
    pub max_position_size_nusd: Decimal256,
    pub limit_orders_timelock: u64,
    pub market_orders_timeout: u64,

    pub is_paused: bool, // prevent opening new trade
    pub is_done: bool,   // prevent any interaction with the contract
    pub vault_fee_p: Decimal256,

    // pair storage
    pub min_lev_pos: HashMap<u64, Decimal256>,
    pub min_leverage: HashMap<u64, u64>,
    pub max_leverage: HashMap<u64, u64>,
}

impl State {
    pub fn new() -> Self {
        Self {
            traders: HashMap::new(),
            open_trades: HashMap::new(),
            open_trades_info: HashMap::new(),
            open_trades_count: HashMap::new(),
            open_limit_orders: Vec::new(),
            open_limit_order_ids: HashMap::new(),
            open_limit_orders_count: HashMap::new(),
            pending_market_orders: HashMap::new(),
            pending_order_ids: HashMap::new(),
            pending_market_open_count: HashMap::new(),
            pending_market_close_count: HashMap::new(),
            pair_traders: HashMap::new(),
            pair_traders_id: HashMap::new(),
            open_interest: HashMap::new(),
            trades_per_block: HashMap::new(),
            max_negative_pnl_on_open_p: Decimal256::from_ratio(40_u64, 100_u64),
            pair_params: HashMap::new(),
            pair_funding_fees: HashMap::new(),
            pair_rollover_fees: HashMap::new(),
            trade_initial_acc_fees: HashMap::new(),
            max_trades_per_pair: 3,
            max_trade_per_block: 5,
            max_pending_market_orders: 5,
            max_gain_p: Decimal256::from_ratio(900_u64, 100_u64),
            max_sl_p: Decimal256::from_ratio(80_u64, 100_u64),
            default_leverage_unlocked: 50_u64,
            spread_reductions_p: [
                Decimal256::from_ratio(15_u64, 100_u64),
                Decimal256::from_ratio(20_u64, 100_u64),
                Decimal256::from_ratio(25_u64, 100_u64),
                Decimal256::from_ratio(30_u64, 100_u64),
                Decimal256::from_ratio(35_u64, 100_u64),
            ],
            max_position_size_nusd: Decimal256::from_atomics(75_000_u128, 0)
                .unwrap(),
            limit_orders_timelock: 30_u64, // 30 blocks
            market_orders_timeout: 30_u64, // 30 blocks

            is_paused: false,
            is_done: false,
            vault_fee_p: Decimal256::from_ratio(10_u64, 100_u64),

            min_lev_pos: HashMap::new(),
            min_leverage: HashMap::new(),
            max_leverage: HashMap::new(),
        }
    }
}
