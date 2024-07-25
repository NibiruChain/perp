use crate::error::ContractError;
use cosmwasm_std::{Addr, Decimal, Deps, Env, Timestamp, Uint128};
use state::{TraderDailyInfo, TRADER_DAILY_INFOS};

pub mod state;

pub fn calculate_fee_amount(
    deps: &Deps,
    env: Env,
    trader: Addr,
    normal_fee_amount_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let trader_daily_info: TraderDailyInfo = TRADER_DAILY_INFOS
        .load(
            deps.storage,
            (trader.to_string(), get_current_day(env.block.time)),
        )
        .ok(TraderDailyInfo::new(0, 0))?;

    if trader_daily_info.fee_multiplier_cache.is_zero() {
        return Ok(normal_fee_amount_collateral);
    }
    Ok(trader_daily_info
        .fee_multiplier_cache
        .checked_mul(Decimal::from_atomics(normal_fee_amount_collateral, 0)?)?
        .to_uint_floor())
}

fn get_current_day(time: Timestamp) -> u64 {
    return time.seconds() / 86400;
}
