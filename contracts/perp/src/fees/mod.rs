use crate::{error::ContractError, utils::u128_to_dec};
use cosmwasm_std::{Addr, Decimal, Deps, Env, Timestamp, Uint128};
use state::{TraderDailyInfo, TRADER_DAILY_INFOS};

pub mod state;

pub fn calculate_fee_amount(
    deps: &Deps,
    env: Env,
    trader: &Addr,
    normal_fee_amount_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let trader_daily_info: TraderDailyInfo = TRADER_DAILY_INFOS
        .load(
            deps.storage,
            (trader.to_string(), get_current_day(env.block.time)),
        )
        .unwrap_or_else(|_| TraderDailyInfo::new());

    if trader_daily_info.fee_multiplier_cache.is_zero() {
        return Ok(normal_fee_amount_collateral);
    }
    Ok(trader_daily_info
        .fee_multiplier_cache
        .checked_mul(u128_to_dec(normal_fee_amount_collateral)?)?
        .to_uint_floor())
}

fn get_current_day(time: Timestamp) -> u64 {
    time.seconds() / 86400
}
