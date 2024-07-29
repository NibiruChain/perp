use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};
use cw_storage_plus::{Item, Map};

pub const FEE_TIERS: Item<[FeeTier; 8]> = Item::new("fee_tiers");
pub const PENDING_GOV_FEES: Map<u64, Uint128> = Map::new("fees");
pub const GROUP_VOLUME_MULTIPLIERS: Map<u64, Decimal> =
    Map::new("group_volume_multipliers");
pub const TRADER_INFOS: Map<(u64, String), TraderInfo> =
    Map::new("trader_infos");
// trader -> day -> TraderDailyInfo
pub const TRADER_DAILY_INFOS: Map<(String, u64), TraderDailyInfo> =
    Map::new("trader_daily_infos");

#[cw_serde]
pub struct FeeTier {
    pub fee_multiplier: Decimal,
    pub points_treshold: Uint128,
}

#[cw_serde]
pub struct TraderInfo {
    pub last_day_updated: Uint128,
    pub trailing_points: Uint128,
}

#[cw_serde]
pub struct TraderDailyInfo {
    pub fee_multiplier_cache: Decimal,
    pub points: Uint128,
}

impl TraderDailyInfo {
    pub fn new() -> Self {
        Self {
            fee_multiplier_cache: Decimal::zero(),
            points: Uint128::zero(),
        }
    }
}
