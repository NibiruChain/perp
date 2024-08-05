pub mod state;

use cosmwasm_std::{Addr, Decimal, DepsMut, Env, Storage, Timestamp, Uint128};
use state::{OiWindowsSettings, OI_WINDOWS_SETTINGS, PAIR_DEPTHS, WINDOWS};

use crate::{
    error::ContractError,
    trade::get_token_price,
    trading::state::{Trade, TradeInfo, TRADE_INFOS},
};

const MAX_WINDOW_COUNT: u64 = 5;

fn get_window_id(timestamp: Timestamp, settings: &OiWindowsSettings) -> u64 {
    (timestamp.seconds() - settings.start_ts) / settings.windows_duration
}

fn get_current_window_id(
    settings: &OiWindowsSettings,
    current_timestamp: Timestamp,
) -> u64 {
    get_window_id(current_timestamp, settings)
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
    env: Env,
    pair_index: u64,
    long: bool,
) -> Result<Uint128, ContractError> {
    let settings = OI_WINDOWS_SETTINGS.load(storage)?;

    if settings.windows_count == 0 {
        return Ok(Uint128::zero());
    }

    let current_window_id = get_current_window_id(&settings, env.block.time);
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
    env: Env,
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
        get_price_impact_oi(storage, env, pair_index, long)?
    } else {
        Uint128::zero()
    };

    _get_trade_price_impact(
        open_price,
        long,
        start_open_interest_usd,
        trade_open_interest_usd,
        Uint128::new(depth),
    )
}

pub fn add_price_impact_open_interest(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    trade_info: TradeInfo,
    position_collateral: Uint128,
) -> Result<(), ContractError> {
    let oi_window_settings = OI_WINDOWS_SETTINGS.load(deps.storage)?;
    let current_window_id =
        get_current_window_id(&oi_window_settings, env.block.time);

    let current_collateral_price =
        get_token_price(&deps.as_ref(), &trade.collateral_index)?;

    let mut oi_delta_usd = convert_collateral_to_usd(
        &trade.collateral_index,
        &trade.pair_index,
        position_collateral,
        current_collateral_price,
    )?;

    let is_partial = trade_info.last_oi_update_ts > Timestamp::from_nanos(0);

    if is_partial
        && (get_window_id(trade_info.last_oi_update_ts, &oi_window_settings)
            >= get_earliest_active_window_id(
                current_window_id,
                oi_window_settings.windows_count,
            ))
    {
        let last_window_oi_usd =
            get_trade_last_window_oi_usd(&trade.user, &trade.pair_index);
        remove_price_impact_open_interest(
            deps,
            env.clone(),
            trade.clone(),
            Uint128::from(last_window_oi_usd),
        )?;

        oi_delta_usd += current_collateral_price
            .checked_mul(Decimal::from_atomics(last_window_oi_usd, 0)?)?
            .checked_div(trade_info.collateral_price_usd)?
            .to_uint_floor();
    }

    // add oi to current window
    let mut current_window = WINDOWS.load(
        deps.storage,
        (
            trade.pair_index,
            current_window_id,
            oi_window_settings.windows_count,
        ),
    )?;

    if trade.long {
        current_window.oi_long_usd += oi_delta_usd;
    } else {
        current_window.oi_short_usd += oi_delta_usd;
    }

    // update trade info
    let mut trade_info = trade_info;
    trade_info.last_oi_update_ts = env.block.time;
    trade_info.collateral_price_usd = current_collateral_price;

    TRADE_INFOS.save(
        deps.storage,
        (trade.user.clone(), trade.index),
        &trade_info,
    )?;

    WINDOWS.save(
        deps.storage,
        (
            trade.pair_index,
            current_window_id,
            oi_window_settings.windows_count,
        ),
        &current_window,
    )?;

    Ok(())
}

pub fn remove_price_impact_open_interest(
    deps: &mut DepsMut,
    env: Env,
    trade: Trade,
    oi_delta_collateral: Uint128,
) -> Result<(), ContractError> {
    let trade_info =
        TRADE_INFOS.load(deps.storage, (trade.user.clone(), trade.index))?;

    let oi_window_settings = OI_WINDOWS_SETTINGS.load(deps.storage)?;

    if oi_delta_collateral.is_zero()
        || trade_info.last_oi_update_ts == Timestamp::from_nanos(0)
    {
        return Ok(());
    }

    let current_window_id =
        get_current_window_id(&oi_window_settings, env.block.time);
    let add_window_id =
        get_window_id(trade_info.last_oi_update_ts, &oi_window_settings);
    let not_outdated =
        is_window_potentially_active(add_window_id, current_window_id);

    let mut oi_delta_usd = convert_collateral_to_usd(
        &trade.collateral_index,
        &trade.pair_index,
        oi_delta_collateral,
        get_token_price(&deps.as_ref(), &trade.collateral_index)?,
    )?;

    if not_outdated {
        let mut window = WINDOWS.load(
            deps.storage,
            (
                trade.pair_index,
                current_window_id,
                oi_window_settings.windows_count,
            ),
        )?;

        let last_window_oi_usd =
            get_trade_last_window_oi_usd(&trade.user, &trade.pair_index);

        oi_delta_usd = if oi_delta_usd > Uint128::from(last_window_oi_usd) {
            Uint128::from(last_window_oi_usd)
        } else {
            oi_delta_usd
        };

        if trade.long {
            window.oi_long_usd = if oi_delta_usd < window.oi_long_usd {
                window.oi_long_usd - oi_delta_usd
            } else {
                Uint128::zero()
            };
        } else {
            window.oi_short_usd = if oi_delta_usd < window.oi_short_usd {
                window.oi_short_usd - oi_delta_usd
            } else {
                Uint128::zero()
            };
        }
        WINDOWS.save(
            deps.storage,
            (
                trade.pair_index,
                current_window_id,
                oi_window_settings.windows_count,
            ),
            &window,
        )?;
    }

    Ok(())
}

fn is_window_potentially_active(
    add_window_id: u64,
    current_window_id: u64,
) -> bool {
    current_window_id - add_window_id < MAX_WINDOW_COUNT
}

fn get_trade_last_window_oi_usd(_trader: &Addr, _index: &u64) -> u64 {
    todo!()
}

fn convert_collateral_to_usd(
    _collateral_index: &u64,
    _pair_index: &u64,
    _position_collateral: Uint128,
    _current_collateral_price: Decimal,
) -> Result<Uint128, ContractError> {
    todo!()
}
