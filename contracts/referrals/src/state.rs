use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, DepsMut, Empty, StdResult, Uint128};
use cw_storage_plus::Map;

#[derive(Default)]
#[cw_serde]
pub struct Tier {
    pub total_rebate: Uint128,
    pub discount_share: Uint128,
}

pub const BASIS_POINTS: Uint128 = Uint128::new(10000);

pub const REFERRER_DISCOUNT_SHARES: Map<Addr, Uint128> = Map::new("referrer_discount_shares");
pub const REFERRER_TIERS: Map<Addr, Uint128> = Map::new("referrer_tiers");

pub const TIERS: Map<u128, Tier> = Map::new("tiers");

pub const CODE_OWNERS: Map<Vec<u8>, Addr> = Map::new("code_owners");
pub const TRADER_REFERRAL_CODES: Map<Addr, Vec<u8>> = Map::new("trader_referral_codes");

pub const REFERRALS_ADMINS: Map<Addr, Empty> = Map::new("referrals_admins");

pub fn add_admin(deps: DepsMut, addr: Addr) -> StdResult<()> {
    REFERRALS_ADMINS.save(deps.storage, addr, &Empty::default())
}

pub fn remove_admin(deps: DepsMut, addr: Addr) {
    REFERRALS_ADMINS.remove(deps.storage, addr)
}

pub fn is_admin(deps: Deps, addr: Addr) -> bool {
    REFERRALS_ADMINS.has(deps.storage, addr)
}
