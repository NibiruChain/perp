use cosmwasm_std::{Decimal, Deps, Storage, Uint128};
use oracle::contract::OracleQueryMsg;

use crate::{
    borrowing::state::{GROUP_OIS, PAIR_OIS},
    error::ContractError,
    pairs::state::{FEES, ORACLE_ADDRESS, PAIRS},
    utils::u128_to_dec,
};

pub(crate) fn get_market_execution_price(
    price: Decimal,
    spread_p: Decimal,
    long: bool,
) -> Decimal {
    let price_diff = price.checked_mul(spread_p).unwrap();
    if long {
        price.checked_add(price_diff).unwrap()
    } else {
        price.checked_sub(price_diff).unwrap()
    }
}

pub(crate) fn get_position_size_collateral(
    collateral_amount: Uint128,
    leverage: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(leverage.checked_mul(collateral_amount)?)
}

pub(crate) fn within_exposure_limits(
    storage: &dyn Storage,
    pair_index: u64,
    group_index: u64,
    collateral_index: u64,
    long: bool,
    position_size_collateral: Uint128,
) -> Result<(), ContractError> {
    let group_ois = GROUP_OIS.load(storage, (collateral_index, group_index))?;
    let pair_ois = PAIR_OIS.load(storage, (collateral_index, pair_index))?;

    let pair_oi_collateral = if long { pair_ois.long } else { pair_ois.short };
    let group_oi_collateral = if long {
        group_ois.long
    } else {
        group_ois.short
    };

    if !(position_size_collateral + pair_oi_collateral <= pair_ois.max
        && position_size_collateral + group_oi_collateral <= group_ois.max)
    {
        return Err(ContractError::ExposureLimitReached);
    }
    Ok(())
}

pub(crate) fn get_position_size_collateral_basis(
    deps: &Deps,
    collateral_index: &u64,
    pair_index: &u64,
    position_size_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let pair = PAIRS.load(deps.storage, *pair_index)?;
    let min_fee = FEES.load(deps.storage, pair.fee_index)?.get_min_fee_usd()?;
    let collateral_price = get_collateral_price_usd(deps, *collateral_index)?;

    let min_fee_collateral = u128_to_dec(min_fee)?
        .checked_div(collateral_price)?
        .to_uint_floor();

    Ok(Uint128::max(position_size_collateral, min_fee_collateral))
}

pub fn get_collateral_price_usd(
    deps: &Deps,
    collateral_index: u64,
) -> Result<Decimal, ContractError> {
    get_collateral_price(deps, &collateral_index)
}
pub fn get_collateral_price(
    deps: &Deps,
    oracle_index: &u64,
) -> Result<Decimal, ContractError> {
    Ok(deps.querier.query_wasm_smart::<Decimal>(
        ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        &OracleQueryMsg::GetCollateralPrice {
            index: *oracle_index,
        },
    )?)
}
