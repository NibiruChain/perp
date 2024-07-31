use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256};
use cw_storage_plus::{Item, Map};
use std::collections::HashMap;

pub const OI_WINDOWS_SETTINGS: Item<OiWindowsSettings> =
    Item::new("oi_windows_settings");
pub const WINDOWS: Map<(u64, u64, u64), PairOi> = Map::new("windows");
pub const PAIR_DEPTHS: Map<u64, PairDepth> = Map::new("pair_depths");

// todo: check why it's not used
// pub const TRADE_PRICE_IMPACT_INFOS: Item<
//     HashMap<(Addr, u32), TradePriceImpactInfo>,
// > = Item::new("trade_price_impact_infos");

#[cw_serde]
pub struct OiWindowsSettings {
    pub start_ts: u64,
    pub windows_duration: u64,
    pub windows_count: u64,
}

#[cw_serde]
pub struct PairOi {
    pub oi_long_usd: Uint128,
    pub oi_short_usd: Uint128,
}

#[cw_serde]
pub struct OiWindowUpdate {
    trader: Addr,
    index: u32,
    windows_duration: u64,
    pair_index: Uint256,
    window_id: Uint256,
    long: bool,
    open_interest_usd: u128, // 1e18 USD
}

#[cw_serde]
pub struct PairDepth {
    pub one_percent_depth_above_usd: u128, // USD
    pub one_percent_depth_below_usd: u128, // USD
}

#[cw_serde]
pub struct TradePriceImpactInfo {
    last_window_oi_usd: u128, // 1e18 USD
    placeholder: u128,
}
