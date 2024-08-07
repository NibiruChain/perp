use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

use crate::error::ContractError;

pub const PAIRS: Map<u64, Pair> = Map::new("pairs");
pub const GROUPS: Map<u64, Group> = Map::new("groups");
pub const FEES: Map<u64, Fee> = Map::new("fees");
pub const PAIR_CUSTOM_MAX_LEVERAGE: Map<u64, Uint128> =
    Map::new("pair_custom_max_leverage");

// todo: check why it's not used
// pub const IS_PAIR_LISTED: Map<String, HashMap<String, bool>> =
//     Map::new("is_pair_listed");
// pub const PAIRS_COUNT: Item<Uint256> = Item::new("pairs_count");
// pub const GROUPS_COUNT: Item<Uint256> = Item::new("groups_count");
// pub const FEES_COUNT: Item<Uint256> = Item::new("fees_count");

pub const ORACLE_ADDRESS: Item<Addr> = Item::new("oracle_address");
pub const STAKING_ADDRESS: Item<Addr> = Item::new("staking_address");
pub const VAULT_ADDRESS: Item<Addr> = Item::new("vault_address");

#[cw_serde]
pub struct Pair {
    pub from: String,
    pub to: String,
    pub spread_p: Decimal,
    pub oracle_index: u64,
    pub group_index: u64,
    pub fee_index: u64,
}

impl Pair {
    pub fn pretty_print(&self) -> String {
        format!("{}-{}", self.from, self.to,)
    }
}

#[cw_serde]
pub struct Group {
    pub name: String,
    pub min_leverage: Uint128,
    pub max_leverage: Uint128,
}

#[cw_serde]
pub struct Fee {
    pub name: String,
    pub open_fee_p: Decimal,   // 1e10 (% of position size)
    pub close_fee_p: Decimal,  // 1e10 (% of position size)
    pub oracle_fee_p: Decimal, // 1e10 (% of position size)
    pub trigger_order_fee_p: Decimal, // 1e10 (% of position size)
    pub min_position_size_usd: Uint128,
}

impl Fee {
    pub fn get_min_fee_usd(&self) -> Result<Uint128, ContractError> {
        let one = Decimal::one();
        let two = one + one;

        Ok((Decimal::from_ratio(self.min_position_size_usd, 0_u64)
            .checked_mul(
                self.open_fee_p.checked_mul(two)? + self.trigger_order_fee_p,
            )?)
        .to_uint_floor())
    }
}
