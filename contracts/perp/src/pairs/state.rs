use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint256};
use cw_storage_plus::{Item, Map};
use std::collections::HashMap;

use crate::error::ContractError;

pub const PAIRS: Map<u64, Pair> = Map::new("pairs");
pub const GROUPS: Map<u64, Group> = Map::new("groups");
pub const FEES: Map<u64, Fee> = Map::new("fees");
pub const IS_PAIR_LISTED: Map<String, HashMap<String, bool>> =
    Map::new("is_pair_listed");
pub const PAIR_CUSTOM_MAX_LEVERAGE: Map<u64, Uint256> =
    Map::new("pair_custom_max_leverage");

pub const PAIRS_COUNT: Item<Uint256> = Item::new("pairs_count");
pub const GROUPS_COUNT: Item<Uint256> = Item::new("groups_count");
pub const FEES_COUNT: Item<Uint256> = Item::new("fees_count");

#[cw_serde]
pub struct Pair {
    pub from: String,
    pub to: String,
    pub spread_p: Uint256, // 1e10
    pub group_index: u64,
    pub fee_index: u64,
}

#[cw_serde]
pub struct Group {
    pub name: String,
    pub job: [u8; 32],
    pub min_leverage: u64,
    pub max_leverage: u64,
}

#[cw_serde]
pub struct Fee {
    name: String,
    open_fee_p: Decimal,          // 1e10 (% of position size)
    close_fee_p: Decimal,         // 1e10 (% of position size)
    oracle_fee_p: Decimal,        // 1e10 (% of position size)
    trigger_order_fee_p: Decimal, // 1e10 (% of position size)
    min_position_size_usd: Decimal, // 1e18 (collateral x leverage, useful for min fee)
}

impl Fee {
    pub fn get_min_fee_usd(&self) -> Result<Decimal, ContractError> {
        let one = Decimal::one();
        let two = one + one;

        return Ok(self.min_position_size_usd
            * (self.open_fee_p.checked_mul(two)? + self.trigger_order_fee_p));
    }
}