pub mod state;
use cosmwasm_std::{Decimal, Storage, Uint128};
use state::{OiWindowsSettings, OI_WINDOWS_SETTINGS, PAIR_DEPTHS, WINDOWS};

use crate::error::ContractError;
struct ConstantsUtils;
impl ConstantsUtils {
    const P_10: f64 = 1e10;
}

fn get_window_id(timestamp: u64, settings: &OiWindowsSettings) -> u64 {
    (timestamp - settings.start_ts) / settings.windows_duration
}

fn get_current_window_id(settings: &OiWindowsSettings) -> u64 {
    get_window_id(current_timestamp(), settings)
}

fn get_earliest_active_window_id(
    current_window_id: u64,
    windows_count: u64,
) -> u64 {
    if current_window_id > windows_count - 1 {
        current_window_id - (windows_count - 1)
    } else {
        0
    }
}

fn is_window_potentially_active(window_id: u64, current_window_id: u64) -> bool {
    current_window_id - window_id < 5
}

fn _get_trade_price_impact(
    open_price: Decimal,
    long: bool,
    start_open_interest_usd: Uint128,
    trade_open_interest_usd: Decimal,
    one_percent_depth_usd: Uint128,
) -> Result<(Decimal, Decimal), ContractError> {
    if one_percent_depth_usd.is_zero() {
        return Ok((Decimal::zero(), open_price));
    }
    let two = Decimal::one() + Decimal::one();

    let price_impact_p = (Decimal::from_atomics(start_open_interest_usd, 0)?
        + trade_open_interest_usd.checked_div(two)?)
    .checked_div(Decimal::from_atomics(one_percent_depth_usd, 0)?)?;

    let price_impact = price_impact_p * open_price;
    let price_after_impact = if long {
        open_price + price_impact
    } else {
        open_price - price_impact
    };

    Ok((price_impact_p, price_after_impact))
}

fn get_price_impact_oi(
    storage: &dyn Storage,
    pair_index: u64,
    long: bool,
) -> Result<Uint128, ContractError> {
    let settings = OI_WINDOWS_SETTINGS.load(storage)?;

    if settings.windows_count == 0 {
        return Ok(Uint128::zero());
    }

    let current_window_id = get_current_window_id(&settings);
    let earliest_active_window_id =
        get_earliest_active_window_id(current_window_id, settings.windows_count);

    let mut active_oi = Uint128::zero();
    for window_id in earliest_active_window_id..=current_window_id {
        let windows = WINDOWS
            .load(storage, (pair_index, window_id, settings.windows_count))?;

        active_oi += if long {
            windows.oi_long_usd
        } else {
            windows.oi_short_usd
        }
    }
    Ok(active_oi)
}

pub fn get_trade_price_impact(
    storage: &dyn Storage,
    open_price: Decimal,
    pair_index: u64,
    long: bool,
    trade_open_interest_usd: Decimal,
) -> Result<(Decimal, Decimal), ContractError> {
    let pair_depth = PAIR_DEPTHS.load(storage, pair_index)?;

    let depth = if long {
        pair_depth.one_percent_depth_above_usd
    } else {
        pair_depth.one_percent_depth_below_usd
    };

    let start_open_interest_usd = if depth > 0 {
        get_price_impact_oi(storage, pair_index, long)?
    } else {
        Uint128::zero()
    };

    return _get_trade_price_impact(
        open_price,
        long,
        start_open_interest_usd,
        trade_open_interest_usd,
        Uint128::new(depth),
    );
}

fn current_timestamp() -> u64 {
    // This is a placeholder. In real implementation, you would get the actual current timestamp.
    1_627_843_200
}
