use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Map;

pub const PAIRS: Map<(u64, u64), BorrowingData> = Map::new("borrowing_data");
pub const PAIR_GROUPS: Map<(u64, u64), Vec<BorrowingPairGroup>> =
    Map::new("borrowing_pair_group");
pub const PAIR_OIS: Map<(u64, u64), OpenInterest> = Map::new("pair_ois");
pub const GROUPS: Map<(u64, u64), BorrowingData> = Map::new("borrowing_data");
pub const GROUP_OIS: Map<(u64, u64), OpenInterest> = Map::new("group_ois");
pub const INITIAL_ACC_FEES: Map<(u64, Addr, u64), BorrowingInitialAccFees> =
    Map::new("initial_acc_fees");

#[cw_serde]
pub struct BorrowingData {
    pub fee_per_block: Decimal, // %
    pub acc_fee_long: Decimal,  // %
    pub acc_fee_short: Decimal, // %
    pub acc_last_updated_block: u64,
    pub fee_exponent: u32,
}

#[cw_serde]
pub struct BorrowingPairGroup {
    pub group_index: u64,
    pub block: u64,
    pub initial_acc_fee_long: Decimal,     // %
    pub initial_acc_fee_short: Decimal,    // %
    pub prev_group_acc_fee_long: Decimal,  // %
    pub prev_group_acc_fee_short: Decimal, // %
    pub pair_acc_fee_long: Decimal,        // %
    pub pair_acc_fee_short: Decimal,       // %
}

#[cw_serde]
pub struct OpenInterest {
    pub long: Uint128, // 1e10 (collateral) - Using Uint128 to represent the wider bit-width type
    pub short: Uint128, // 1e10 (collateral)
    pub max: Uint128,  // 1e10 (collateral)
}

#[cw_serde]
pub struct BorrowingInitialAccFees {
    pub acc_pair_fee: Decimal,  // %
    pub acc_group_fee: Decimal, // %
    pub block: u64,
}

#[cw_serde]
pub struct BorrowingPairParams {
    group_index: u64,
    fee_per_block: Decimal, // %
    fee_exponent: u64,
    max_oi: Uint128, // 1e10 (collateral) - Using u128 to represent the wider bit-width type
}

#[cw_serde]
pub struct BorrowingGroupParams {
    fee_per_block: Decimal, // %
    max_oi: u128,           // 1e10 (collateral)
    fee_exponent: u64,
}

#[cw_serde]
pub struct BorrowingFeeInput {
    pub collateral_index: u64,
    pub trader: Addr, // address is represented as String in Rust
    pub pair_index: u64,
    pub index: u64,
    pub long: bool,
    pub collateral: Uint128, // 1e18 | 1e6 (collateral) - Using Uint128 to represent the wider bit-width type
    pub leverage: Uint128, // 1e3 - Using Uint128 to represent the wider bit-width type
}

#[cw_serde]
pub struct LiqPriceInput {
    collateral_index: u8,
    trader: Addr, // address is represented as Addr in Rust
    pair_index: u16,
    index: u32,
    open_price: u64, // 1e10
    long: bool,
    collateral: Uint128, // 1e18 | 1e6 (collateral) - Using Uint128 to represent the wider bit-width type
    leverage: Uint128, // 1e3 - Using Uint128 to represent the wider bit-width type
    use_borrowing_fees: bool,
}

#[cw_serde]
pub struct PendingBorrowingAccFeesInput {
    pub acc_fee_long: Decimal,  // %
    pub acc_fee_short: Decimal, // %
    pub oi_long: Uint128, // 1e18 | 1e6 - Using Uint128 to represent the wider bit-width type
    pub oi_short: Uint128, // 1e18 | 1e6 - Using Uint128 to represent the wider bit-width type
    pub fee_per_block: Decimal, // 1e10
    pub current_block: u64,
    pub acc_last_updated_block: u64,
    pub max_oi: Uint128, // 1e10 (collateral) - Using u128 to represent the wider bit-width type
    pub fee_exponent: u32,
}
