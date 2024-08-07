use cosmwasm_std::{Decimal, Int128, Uint128};

use crate::error::ContractError;

pub fn u128_to_i128(u: Uint128) -> Result<Int128, ContractError> {
    Int128::try_from(u.u128() as i128)
        .map_err(|_| ContractError::ConversionOverflow)
}

pub fn u128_to_dec(u: Uint128) -> Result<Decimal, ContractError> {
    Decimal::from_atomics(u.u128(), 0_u32)
        .map_err(|_| ContractError::ConversionOverflow)
}
