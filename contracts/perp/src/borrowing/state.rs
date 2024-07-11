use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint256};
use cw_storage_plus::Item;
use std::collections::HashMap;

pub const PAIRS: Item<HashMap<(u8, u16), BorrowingData>> =
    Item::new("borrowing_data");
pub const PAIR_GROUPS: Item<HashMap<(u8, u16), BorrowingPairGroup>> =
    Item::new("borrowing_pair_group");
pub const PAIR_OIS: Item<HashMap<(u8, u16), OpenInterest>> =
    Item::new("open_interest");
pub const GROUPS: Item<HashMap<(u8, u16), BorrowingData>> =
    Item::new("borrowing_data");
pub const GROUP_OIS: Item<HashMap<(u8, u16), OpenInterest>> =
    Item::new("open_interest");
pub const INITIAL_ACC_FEES: Item<
    HashMap<(u8, Addr, u32), BorrowingInitialAccFees>,
> = Item::new("initial_acc_fees");

#[cw_serde]
pub struct BorrowingData {
    fee_per_block: u32, // 1e10 (%)
    acc_fee_long: u64,  // 1e10 (%)
    acc_fee_short: u64, // 1e10 (%)
    acc_last_updated_block: u64,
    fee_exponent: u64,
}

#[cw_serde]
pub struct BorrowingPairGroup {
    group_index: u16,
    block: u64,
    initial_acc_fee_long: u64,     // 1e10 (%)
    initial_acc_fee_short: u64,    // 1e10 (%)
    prev_group_acc_fee_long: u64,  // 1e10 (%)
    prev_group_acc_fee_short: u64, // 1e10 (%)
    pair_acc_fee_long: u64,        // 1e10 (%)
    pair_acc_fee_short: u64,       // 1e10 (%)
    placeholder: u64,              // might be useful later
}

#[cw_serde]
pub struct OpenInterest {
    long: u128, // 1e10 (collateral) - Using u128 to represent the wider bit-width type
    short: u128, // 1e10 (collateral)
    max: u128,  // 1e10 (collateral)
    placeholder: u64, // might be useful later
}

#[cw_serde]
pub struct BorrowingInitialAccFees {
    acc_pair_fee: u64,  // 1e10 (%)
    acc_group_fee: u64, // 1e10 (%)
    block: u64,
}

#[cw_serde]
pub struct BorrowingPairParams {
    group_index: u16,
    fee_per_block: u32, // 1e10 (%)
    fee_exponent: u64,
    max_oi: u128, // 1e10 (collateral) - Using u128 to represent the wider bit-width type
}

#[cw_serde]
pub struct BorrowingGroupParams {
    fee_per_block: u32, // 1e10 (%)
    max_oi: u128,       // 1e10 (collateral)
    fee_exponent: u64,
}

#[cw_serde]
pub struct BorrowingFeeInput {
    collateral_index: u8,
    trader: Addr, // address is represented as String in Rust
    pair_index: u16,
    index: u32,
    long: bool,
    collateral: Uint256, // 1e18 | 1e6 (collateral) - Using Uint256 to represent the wider bit-width type
    leverage: Uint256, // 1e3 - Using Uint256 to represent the wider bit-width type
}

#[cw_serde]
pub struct LiqPriceInput {
    collateral_index: u8,
    trader: Addr, // address is represented as Addr in Rust
    pair_index: u16,
    index: u32,
    open_price: u64, // 1e10
    long: bool,
    collateral: Uint256, // 1e18 | 1e6 (collateral) - Using Uint256 to represent the wider bit-width type
    leverage: Uint256, // 1e3 - Using Uint256 to represent the wider bit-width type
    use_borrowing_fees: bool,
}

#[cw_serde]
pub struct PendingBorrowingAccFeesInput {
    acc_fee_long: u64,               // 1e10 (%)
    acc_fee_short: u64,              // 1e10 (%)
    oi_long: Uint256, // 1e18 | 1e6 - Using Uint256 to represent the wider bit-width type
    oi_short: Uint256, // 1e18 | 1e6 - Using Uint256 to represent the wider bit-width type
    fee_per_block: u32, // 1e10
    current_block: Uint256, // Using Uint256 to represent the wider bit-width type
    acc_last_updated_block: Uint256, // Using Uint256 to represent the wider bit-width type
    max_oi: u128, // 1e10 (collateral) - Using u128 to represent the wider bit-width type
    fee_exponent: u64,
    collateral_precision: u128, // Using u128 to represent the wider bit-width type
}
