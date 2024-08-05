use cosmwasm_std::{Addr, Decimal, Deps, Env, Storage, Uint128};
use state::{
    BorrowingData, BorrowingInitialAccFees, OpenInterest,
    PendingBorrowingAccFeesInput, GROUPS, GROUP_OIS, INITIAL_ACC_FEES, PAIRS,
    PAIR_GROUPS, PAIR_OIS,
};

use crate::{
    constants::LIQ_THRESHOLD_P, error::ContractError,
    fees::calculate_fee_amount, pairs::state::FEES, trade::get_collateral_price,
    trading::state::Trade,
};

pub mod state;

pub fn handle_trade_borrowing(
    env: Env,
    sender: Addr,
    trade_index: u64,
    storage: &mut dyn Storage,
    collateral_index: u64,
    pair_index: u64,
    position_collateral: Uint128,
    open: bool,
    long: bool,
) -> Result<(), ContractError> {
    let block_number = env.block.height;
    let group_index =
        get_borrowing_pair_group_index(storage, collateral_index, pair_index);

    set_pair_pending_acc_fees(
        storage,
        collateral_index,
        pair_index,
        block_number,
    )?;
    set_group_pending_acc_fees(
        storage,
        collateral_index,
        group_index,
        block_number,
    )?;

    update_pair_oi(
        storage,
        collateral_index,
        pair_index,
        long,
        open,
        position_collateral,
    )?;
    update_group_oi(
        storage,
        collateral_index,
        group_index,
        long,
        open,
        position_collateral,
    )?;

    if open {
        reset_trade_borrowing_fees(
            sender,
            storage,
            collateral_index,
            trade_index,
            pair_index,
            group_index,
            long,
            block_number,
        )?;
    }
    Ok(())
}

fn reset_trade_borrowing_fees(
    sender: Addr,
    storage: &mut dyn Storage,
    collateral_index: u64,
    trade_index: u64,
    pair_index: u64,
    group_index: u64,
    long: bool,
    current_block: u64,
) -> Result<(), ContractError> {
    let pair_borrowing_data = get_borrowing_pair_pending_acc_fees(
        storage,
        collateral_index,
        pair_index,
        current_block,
    )?;

    let group_borrowing_data = get_borrowing_group_pending_acc_fees(
        storage,
        collateral_index,
        group_index,
        current_block,
    )?;

    INITIAL_ACC_FEES.save(
        storage,
        (collateral_index, sender, trade_index),
        &BorrowingInitialAccFees {
            acc_pair_fee: if long {
                pair_borrowing_data.acc_fee_long
            } else {
                pair_borrowing_data.acc_fee_short
            },
            acc_group_fee: if long {
                group_borrowing_data.acc_fee_long
            } else {
                group_borrowing_data.acc_fee_short
            },
            block: current_block,
        },
    )?;

    Ok(())
}

fn update_group_oi(
    storage: &mut dyn Storage,
    collateral_index: u64,
    group_index: u64,
    long: bool,
    open: bool,
    position_collateral: Uint128,
) -> Result<OpenInterest, ContractError> {
    let mut group_oi =
        GROUP_OIS.load(storage, (collateral_index, group_index))?;
    update_oi(&mut group_oi, long, open, position_collateral);

    GROUP_OIS.save(storage, (collateral_index, group_index), &group_oi)?;
    Ok(group_oi)
}

fn update_pair_oi(
    storage: &mut dyn Storage,
    collateral_index: u64,
    pair_index: u64,
    long: bool,
    open: bool,
    position_collateral: Uint128,
) -> Result<OpenInterest, ContractError> {
    let mut pair_oi = PAIR_OIS.load(storage, (collateral_index, pair_index))?;
    update_oi(&mut pair_oi, long, open, position_collateral);

    PAIR_OIS.save(storage, (collateral_index, pair_index), &pair_oi)?;
    Ok(pair_oi)
}

/// Function to update a borrowing pair/group open interest
fn update_oi(
    oi: &mut OpenInterest,
    long: bool,
    increase: bool,
    amount_collateral: Uint128,
) -> (Uint128, Uint128, Uint128) {
    let delta = amount_collateral;

    if long {
        if increase {
            oi.long += delta;
        } else if delta > oi.long {
            oi.long = Uint128::zero();
        } else {
            oi.long -= delta;
        }
    } else if increase {
        oi.short += delta;
    } else if delta > oi.short {
        oi.short = Uint128::zero();
    } else {
        oi.short -= delta;
    }

    (oi.long, oi.short, delta)
}

fn set_pair_pending_acc_fees(
    storage: &mut dyn Storage,
    collateral_index: u64,
    pair_index: u64,
    block_number: u64,
) -> Result<(), ContractError> {
    let pair = get_borrowing_pair_pending_acc_fees(
        storage,
        collateral_index,
        pair_index,
        block_number,
    )?;

    Ok(PAIRS.save(storage, (collateral_index, pair_index), &pair)?)
}

fn set_group_pending_acc_fees(
    storage: &mut dyn Storage,
    collateral_index: u64,
    group_index: u64,
    block_number: u64,
) -> Result<(), ContractError> {
    let group = get_borrowing_group_pending_acc_fees(
        storage,
        collateral_index,
        group_index,
        block_number,
    )?;

    Ok(GROUPS.save(storage, (collateral_index, group_index), &group)?)
}

fn get_borrowing_group_pending_acc_fees(
    storage: &mut dyn Storage,
    collateral_index: u64,
    group_index: u64,
    block_number: u64,
) -> Result<BorrowingData, ContractError> {
    let mut group = GROUPS.load(storage, (collateral_index, group_index))?;
    let group_oi = GROUP_OIS.load(storage, (collateral_index, group_index))?;

    let input = PendingBorrowingAccFeesInput {
        acc_fee_long: group.acc_fee_long,
        acc_fee_short: group.acc_fee_short,
        oi_long: group_oi.long,
        oi_short: group_oi.short,
        fee_per_block: group.fee_per_block,
        current_block: block_number,
        acc_last_updated_block: group.acc_last_updated_block,
        max_oi: group_oi.max,
        fee_exponent: group.fee_exponent,
    };

    let (acc_fee_long, acc_fee_short, _pair_acc_fee_delta) =
        get_borrowing_pending_acc_fees(input)?;

    group.acc_fee_long = acc_fee_long;
    group.acc_fee_short = acc_fee_short;
    group.acc_last_updated_block = block_number;

    Ok(group)
}

fn get_borrowing_pair_pending_acc_fees(
    storage: &dyn Storage,
    collateral_index: u64,
    pair_index: u64,
    block_number: u64,
) -> Result<BorrowingData, ContractError> {
    let mut pair = PAIRS.load(storage, (collateral_index, pair_index))?;

    let (pair_oi_long, pair_oi_short) = get_pair_ois_collateral(
        storage,
        collateral_index,
        pair_index,
        block_number,
    )?;

    let input = PendingBorrowingAccFeesInput {
        acc_fee_long: pair.acc_fee_long,
        acc_fee_short: pair.acc_fee_short,
        oi_long: pair_oi_long,
        oi_short: pair_oi_short,
        fee_per_block: pair.fee_per_block,
        current_block: block_number,
        acc_last_updated_block: pair.acc_last_updated_block,
        max_oi: PAIR_OIS.load(storage, (collateral_index, pair_index))?.max,
        fee_exponent: pair.fee_exponent,
    };

    let (acc_fee_long, acc_fee_short, _pair_acc_fee_delta): (u64, u64, u64) =
        get_borrowing_pending_acc_fees(input)?;

    pair.acc_fee_long = acc_fee_long;
    pair.acc_fee_short = acc_fee_short;
    pair.acc_last_updated_block = block_number;

    Ok(pair)
}

fn get_pair_ois_collateral(
    storage: &dyn Storage,
    collateral_index: u64,
    pair_index: u64,
    _block_number: u64,
) -> Result<(Uint128, Uint128), ContractError> {
    let pair_oi = PAIR_OIS.load(storage, (collateral_index, pair_index))?;

    Ok((pair_oi.long, pair_oi.short))
}

/// Function that returns the new acc borrowing fees and delta between two blocks (for pairs and groups)
fn get_borrowing_pending_acc_fees(
    input: PendingBorrowingAccFeesInput,
) -> Result<(u64, u64, u64), ContractError> {
    if input.current_block < input.acc_last_updated_block {
        return Err(ContractError::BlockOrder);
    }
    let more_shorts = input.oi_long < input.oi_short;
    let net_oi = if more_shorts {
        input.oi_short - input.oi_long
    } else {
        input.oi_long - input.oi_short
    };

    let delta = if !input.max_oi.is_zero() && input.fee_exponent > 0 {
        input
            .fee_per_block
            .checked_mul(Decimal::from_atomics(
                input.current_block - input.acc_last_updated_block,
                0,
            )?)?
            .checked_mul(
                Decimal::from_ratio(net_oi, input.max_oi)
                    .pow(input.fee_exponent),
            )?
            .to_uint_floor()
    } else {
        Uint128::zero()
    };

    if delta > u64::MAX.into() {
        return Err(ContractError::Overflow);
    }

    let delta = delta.u128() as u64;

    let acc_fee_long = if !more_shorts {
        input.acc_fee_long + delta
    } else {
        input.acc_fee_long
    };

    let acc_fee_short = if more_shorts {
        input.acc_fee_short + delta
    } else {
        input.acc_fee_short
    };

    Ok((acc_fee_long, acc_fee_short, delta))
}

fn get_borrowing_pair_group_index(
    storage: &dyn Storage,
    collateral_index: u64,
    pair_index: u64,
) -> u64 {
    PAIR_GROUPS
        .load(storage, (collateral_index, pair_index))
        .map(|x| x[x.len() - 1].group_index)
        .unwrap_or(0_u64)
}

pub fn get_trade_liquidation_price_with_fees(
    deps: &Deps,
    env: Env,
    trade: Trade,
    use_borrowing_fees: bool,
) -> Result<Decimal, ContractError> {
    let pair =
        crate::pairs::state::PAIRS.load(deps.storage, trade.pair_index)?;
    let fee = FEES.load(deps.storage, pair.fee_index)?;

    let closing_fees_collateral = Decimal::from_ratio(
        get_position_size_collateral_basis(
            deps,
            trade.collateral_index,
            trade.collateral_amount,
            fee.min_position_size_usd,
        )?,
        0_u64,
    )
    .checked_mul(fee.close_fee_p.checked_add(fee.trigger_order_fee_p)?)?
    .to_uint_floor();

    let borrowing_fees_collateral = if use_borrowing_fees {
        calculate_fee_amount(deps, env, &trade.user, closing_fees_collateral)?
    } else {
        Uint128::zero()
    };

    get_trade_liquidation_price(
        trade.open_price,
        trade.long,
        trade.collateral_amount,
        trade.leverage,
        closing_fees_collateral.checked_add(borrowing_fees_collateral)?,
    )
}

fn get_position_size_collateral_basis(
    deps: &Deps,
    collateral_index: u64,
    position_size_collateral: Uint128,
    min_position_size_usd: Uint128,
) -> Result<Uint128, ContractError> {
    let min_position_size_collateral = get_min_position_size_collateral(
        deps,
        collateral_index,
        min_position_size_usd,
    )?;

    if position_size_collateral > min_position_size_collateral {
        Ok(position_size_collateral)
    } else {
        Ok(min_position_size_collateral)
    }
}

fn get_min_position_size_collateral(
    deps: &Deps,
    collateral_index: u64,
    min_position_size_usd: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(Decimal::from_ratio(min_position_size_usd, 0_u64)
        .checked_div(get_collateral_price(deps, &collateral_index)?)?
        .to_uint_floor())
}

pub fn get_trade_liquidation_price(
    open_price: Decimal,
    long: bool,
    collateral: Uint128,
    leverage: Uint128,
    fees_collateral: Uint128,
) -> Result<Decimal, ContractError> {
    let collateral_liq_negative_pnl =
        LIQ_THRESHOLD_P.checked_mul(Decimal::from_ratio(collateral, 0_u64))?;

    let liq_price_distance = open_price
        .checked_mul(
            collateral_liq_negative_pnl
                .checked_sub(Decimal::from_ratio(fees_collateral, 0_u64))?,
        )?
        .checked_div(Decimal::from_ratio(collateral, 0_u64))?
        .checked_div(Decimal::from_ratio(leverage, 0_u64))?;

    let liq_price = if long {
        open_price.checked_sub(liq_price_distance)?
    } else {
        open_price.checked_add(liq_price_distance)?
    };

    Ok(liq_price)
}
