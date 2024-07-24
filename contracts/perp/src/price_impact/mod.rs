pub mod state;
use cosmwasm_std::{Addr, Decimal, DepsMut, Storage, Timestamp, Uint128};
use state::{OiWindowsSettings, OI_WINDOWS_SETTINGS, PAIR_DEPTHS, WINDOWS};

use crate::{
    error::ContractError,
    trade::get_token_price,
    trading::state::{Trade, TradeInfo},
};
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

fn current_timestamp() -> u64 {
    todo!()
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
    trade_open_interest_usd: Uint128,
    one_percent_depth_usd: Uint128,
) -> Result<(Decimal, Decimal), ContractError> {
    if one_percent_depth_usd.is_zero() {
        return Ok((Decimal::zero(), open_price));
    }

    let price_impact_p = Decimal::from_ratio(
        start_open_interest_usd
            + trade_open_interest_usd.checked_div(2_u64.into())?,
        one_percent_depth_usd,
    );

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
    trade_open_interest_usd: Uint128,
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

pub fn add_price_impact_open_interest(
    deps: &DepsMut,
    trade: Trade,
    trade_info: TradeInfo,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    let oi_window_settings = OI_WINDOWS_SETTINGS.load(deps.storage)?;
    let current_window_id = get_current_window_id(&oi_window_settings);

    let current_collateral_price =
        get_token_price(&deps, trade.collateral_index)?;
    let oi_delta_usd = convert_collateral_to_usd(
        trade.collateral_index,
        trade.pair_index,
        position_collateral,
        current_collateral_price,
    )?;

    let is_partial = trade_info.last_oi_update_ts > Timestamp::from_nanos(0);

    if is_partial
        && (get_window_id(
            trade_info.last_oi_update_ts.seconds(),
            &oi_window_settings,
        ) >= get_earliest_active_window_id(
            current_window_id,
            oi_window_settings.windows_count,
        ))
    {
        let last_window_oi_usd =
            get_trade_last_window_oi_usd(trade.user, trade.pair_index);
        remove_price_impact_open_interest(
            trade.user,
            trade.pair_index,
            last_window_oi_usd,
        );
    }

    Ok(())
}

fn remove_price_impact_open_interest(
    trader: Addr,
    index: u64,
    last_window_oi_usd: u64,
) -> u64 {
    todo!()
}

fn get_trade_last_window_oi_usd(trader: Addr, index: u64) -> u64 {
    todo!()
}

fn convert_collateral_to_usd(
    collateral_index: u64,
    pair_index: u64,
    position_collateral: Uint128,
    current_collateral_price: Decimal,
) -> Result<u64, ContractError> {
    todo!()
}
