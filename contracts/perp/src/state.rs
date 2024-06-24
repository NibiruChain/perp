use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
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
    pub position_size_nusd: u128,
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

#[cw_serde]
pub struct State {
    pub traders: HashMap<Addr, Trader>,
    pub open_trades: HashMap<(Addr, u64, u64), Trade>,
    pub open_trades_info: HashMap<(Addr, u64, u64), TradeInfo>,
    pub open_limit_orders: HashMap<(Addr, u64, u64), OpenLimitOrder>,
    pub pending_market_orders: HashMap<u64, PendingMarketOrder>,
    pub pending_nft_orders: HashMap<u64, PendingNftOrder>,
    pub pair_params: HashMap<u64, PairParams>,
    pub pair_funding_fees: HashMap<u64, PairFundingFees>,
    pub pair_rollover_fees: HashMap<u64, PairRolloverFees>,
    pub trade_initial_acc_fees: HashMap<(Addr, u64, u64), TradeInitialAccFees>,
    pub spread_reductions_p: Vec<u64>,
    pub max_trades_per_pair: u64,
    pub max_pending_market_orders: u64,
    pub max_open_limit_orders_per_pair: u64,
    pub max_sl_p: u64,
    pub max_gain_p: u64,
    pub default_leverage_unlocked: u64,
    pub max_open_interest_nusd: HashMap<u64, [u128; 3]>,
    pub trades_per_block: HashMap<u64, u64>,
    pub nft_last_success: HashMap<u64, u64>,
    pub is_trading_contract: HashMap<Addr, bool>,
}

impl State {
    pub fn new() -> Self {
        Self {
            traders: HashMap::new(),
            open_trades: HashMap::new(),
            open_trades_info: HashMap::new(),
            open_limit_orders: HashMap::new(),
            pending_market_orders: HashMap::new(),
            pending_nft_orders: HashMap::new(),
            pair_params: HashMap::new(),
            pair_funding_fees: HashMap::new(),
            pair_rollover_fees: HashMap::new(),
            trade_initial_acc_fees: HashMap::new(),
            spread_reductions_p: vec![15, 20, 25, 30, 35],
            max_trades_per_pair: 3,
            max_pending_market_orders: 5,
            max_open_limit_orders_per_pair: 3,
            max_sl_p: 80,
            max_gain_p: 900,
            default_leverage_unlocked: 50,
            max_open_interest_nusd: HashMap::new(),
            trades_per_block: HashMap::new(),
            nft_last_success: HashMap::new(),
            is_trading_contract: HashMap::new(),
        }
    }
}
