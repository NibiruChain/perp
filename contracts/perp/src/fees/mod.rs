use crate::{
    constants::GOV_PRICE_COLLATERAL_INDEX,
    error::ContractError,
    fees::state::{PENDING_GOV_FEES, VAULT_CLOSING_FEE_P},
    pairs::state::{
        FEES, ORACLE_ADDRESS, PAIRS, STAKING_ADDRESS, VAULT_ADDRESS,
    },
    trade::get_token_price,
    trading::state::{OpenOrderType, PendingOrderType, Trade, COLLATERALS},
    trading::utils::get_position_size_collateral_basis,
    utils::u128_to_dec,
};

use cosmwasm_std::{
    Addr, BankMsg, BlockInfo, Coin, Decimal, Deps, DepsMut, Timestamp, Uint128,
};

use state::{TraderDailyInfo, TRADER_DAILY_INFOS};

pub mod state;

pub fn calculate_fee_amount(
    deps: &Deps,
    block: &BlockInfo,
    trader: &Addr,
    normal_fee_amount_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let trader_daily_info: TraderDailyInfo = TRADER_DAILY_INFOS
        .load(
            deps.storage,
            (trader.to_string(), get_current_day(block.time)),
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

pub(crate) fn process_opening_fees(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trade: Trade,
    position_size_collateral: Uint128,
    order_type: OpenOrderType,
) -> Result<(Vec<BankMsg>, Uint128), ContractError> {
    let gov_price_collateral =
        get_token_price(&deps.as_ref(), &GOV_PRICE_COLLATERAL_INDEX)?;

    let position_size_collateral = get_position_size_collateral_basis(
        &deps.as_ref(),
        &trade.collateral_index,
        &trade.pair_index,
        position_size_collateral,
    )?;

    // todo: fee tier points
    let mut total_fees_collateral = Uint128::zero();
    let reward1 = Uint128::zero();
    if false {
        // handle referral fees
        total_fees_collateral += reward1;
    }

    let gov_fee_collateral: Uint128 = distribute_gov_fee_collateral(
        deps,
        block,
        &trade.collateral_index,
        trade.user.clone(),
        &trade.pair_index,
        position_size_collateral,
        gov_price_collateral,
        Decimal::from_ratio(reward1, 2_u64).to_uint_floor(),
    )?;

    let reward2 = calculate_fee_amount(
        &deps.as_ref(),
        block,
        &trade.user,
        u128_to_dec(position_size_collateral)?
            .checked_mul(pair_trigger_order_fee(
                &deps.as_ref(),
                trade.pair_index,
            )?)?
            .to_uint_floor(),
    )?;

    total_fees_collateral +=
        gov_fee_collateral.checked_mul(2_u64.into())? + reward2;

    let mut msgs: Vec<BankMsg> = vec![];
    let reward3: Uint128;
    if order_type != OpenOrderType::MARKET {
        reward3 =
            Decimal::from_ratio(reward2.checked_mul(2_u64.into())?, 10_u64)
                .to_uint_floor();

        msgs.push(distribute_trigger_reward(deps, reward3, trade.clone())?);
    } else {
        reward3 = Uint128::zero();
    }

    msgs.push(distribute_staking_reward(
        deps,
        gov_fee_collateral + reward2 - reward3,
        &trade,
    )?);

    Ok((msgs, total_fees_collateral))
}

pub(crate) fn process_closing_fees(
    deps: &mut DepsMut,
    block: &BlockInfo,
    trade: Trade,
    position_size_collateral: Uint128,
    order_type: PendingOrderType,
) -> Result<(Vec<BankMsg>, Uint128, Uint128, Uint128, Uint128), ContractError> {
    // 1. Calculate closing fees
    let position_size_collateral = get_position_size_collateral_basis(
        &deps.as_ref(),
        &trade.collateral_index,
        &trade.pair_index,
        position_size_collateral,
    )?;

    let mut closing_fee_collateral = if order_type != PendingOrderType::LiqClose
    {
        Decimal::from_ratio(position_size_collateral, 1u64)
            .checked_mul(pair_close_fee(&deps.as_ref(), trade.pair_index)?)?
            .to_uint_floor()
    } else {
        Decimal::from_ratio(trade.collateral_amount, 1u64)
            .checked_mul(Decimal::percent(5))?
            .to_uint_floor()
    };

    let mut trigger_fee_collateral = if order_type != PendingOrderType::LiqClose
    {
        Decimal::from_ratio(position_size_collateral, 1u64)
            .checked_mul(pair_trigger_order_fee(
                &deps.as_ref(),
                trade.pair_index,
            )?)?
            .to_uint_floor()
    } else {
        closing_fee_collateral
    };

    // todo: fee tier points
    if order_type != PendingOrderType::LiqClose {
        closing_fee_collateral = calculate_fee_amount(
            &deps.as_ref(),
            block,
            &trade.user,
            closing_fee_collateral,
        )?;
        trigger_fee_collateral = calculate_fee_amount(
            &deps.as_ref(),
            block,
            &trade.user,
            trigger_fee_collateral,
        )?;
    }

    // 3. Calculate vault fee and GNS staking fee
    let (vault_closing_fee_collateral, gov_staking_fee_collateral) =
        get_closing_fees_collateral(
            &deps.as_ref(),
            closing_fee_collateral,
            trigger_fee_collateral,
            order_type.clone(),
        )?;

    // 4. If trade collateral is enough to pay min fee, distribute closing fees (otherwise charged as negative PnL)
    let mut collateral_left_in_storage = trade.collateral_amount;
    let mut msgs: Vec<BankMsg> = vec![];

    let total_fees = gov_staking_fee_collateral + vault_closing_fee_collateral;

    if collateral_left_in_storage >= total_fees {
        msgs.push(distribute_vault_reward(
            deps,
            vault_closing_fee_collateral,
            &trade,
        )?);
        msgs.push(distribute_staking_reward(
            deps,
            gov_staking_fee_collateral,
            &trade,
        )?);

        if order_type != PendingOrderType::Market {
            msgs.push(distribute_trigger_reward(
                deps,
                trigger_fee_collateral,
                trade,
            )?);
        }

        collateral_left_in_storage =
            collateral_left_in_storage.checked_sub(total_fees)?
    }

    Ok((
        msgs,
        vault_closing_fee_collateral,
        gov_staking_fee_collateral,
        trigger_fee_collateral,
        collateral_left_in_storage,
    ))
}

fn distribute_vault_reward(
    deps: &mut DepsMut,
    reward: Uint128,
    trade: &Trade,
) -> Result<BankMsg, ContractError> {
    Ok(BankMsg::Send {
        to_address: VAULT_ADDRESS.load(deps.storage)?.to_string(),
        amount: vec![Coin::new(
            reward,
            COLLATERALS.load(deps.storage, trade.collateral_index)?,
        )],
    })
}

fn distribute_staking_reward(
    deps: &mut DepsMut,
    reward: Uint128,
    trade: &Trade,
) -> Result<BankMsg, ContractError> {
    Ok(BankMsg::Send {
        to_address: STAKING_ADDRESS.load(deps.storage)?.to_string(),
        amount: vec![Coin::new(
            reward,
            COLLATERALS.load(deps.storage, trade.collateral_index)?,
        )],
    })
}

fn distribute_trigger_reward(
    deps: &mut DepsMut,
    trigger_fee_collateral: Uint128,
    trade: Trade,
) -> Result<BankMsg, ContractError> {
    let gov_price_collateral =
        get_token_price(&deps.as_ref(), &GOV_PRICE_COLLATERAL_INDEX)?;
    let trigger_fee_gov = gov_price_collateral
        .checked_div(u128_to_dec(trigger_fee_collateral)?)?
        .to_uint_floor();

    let message = BankMsg::Send {
        to_address: ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        amount: vec![Coin::new(
            trigger_fee_gov,
            COLLATERALS.load(deps.storage, trade.collateral_index)?,
        )],
    };
    Ok(message)
}

fn get_closing_fees_collateral(
    deps: &Deps,
    closing_fee_collateral: Uint128,
    trigger_fee_collateral: Uint128,
    order_type: PendingOrderType,
) -> Result<(Uint128, Uint128), ContractError> {
    let vault_closing_fee_p = VAULT_CLOSING_FEE_P.load(deps.storage)?;

    let vault_closing_fee_collateral =
        Decimal::from_ratio(closing_fee_collateral, 1u64)
            .checked_mul(vault_closing_fee_p)?
            .to_uint_floor();

    let gov_staking_fee_collateral: Uint128 =
        if order_type == PendingOrderType::Market {
            trigger_fee_collateral
        } else {
            Decimal::from_ratio(closing_fee_collateral, 1u64)
                .checked_mul(Decimal::one().checked_sub(vault_closing_fee_p)?)?
                .to_uint_floor()
        };
    Ok((vault_closing_fee_collateral, gov_staking_fee_collateral))
}

fn distribute_gov_fee_collateral(
    deps: &mut DepsMut,
    block: &BlockInfo,
    collateral_index: &u64,
    user: Addr,
    pair_index: &u64,
    position_size_collateral: Uint128,
    gov_price_collateral: Decimal,
    referral_fee_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let gov_fee_collateral = get_gov_fee_collateral(
        &deps.as_ref(),
        block,
        user.clone(),
        *pair_index,
        position_size_collateral,
        gov_price_collateral,
    )? - referral_fee_collateral;

    distribute_exact_gov_fee_collateral(
        deps,
        *collateral_index,
        user,
        gov_fee_collateral,
    )
}

fn get_gov_fee_collateral(
    deps: &Deps,
    block: &BlockInfo,
    user: Addr,
    pair_index: u64,
    position_size_collateral: Uint128,
    _gov_price_collateral: Decimal,
) -> Result<Uint128, ContractError> {
    let pair = PAIRS.load(deps.storage, pair_index)?;
    let fee = FEES.load(deps.storage, pair.fee_index)?;

    calculate_fee_amount(
        deps,
        block,
        &user,
        u128_to_dec(position_size_collateral)?
            .checked_mul(fee.open_fee_p)?
            .to_uint_floor(),
    )
}

fn distribute_exact_gov_fee_collateral(
    deps: &mut DepsMut,
    collateral_index: u64,
    _user: Addr,
    gov_fee_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let mut pending_gov_fees =
        PENDING_GOV_FEES.load(deps.as_ref().storage, collateral_index)?;
    pending_gov_fees += gov_fee_collateral;
    PENDING_GOV_FEES.save(deps.storage, collateral_index, &pending_gov_fees)?;
    Ok(pending_gov_fees)
}

fn pair_trigger_order_fee(
    deps: &Deps,
    pair_index: u64,
) -> Result<Decimal, ContractError> {
    let pair = PAIRS.load(deps.storage, pair_index)?;
    let fee = FEES.load(deps.storage, pair.fee_index)?;

    Ok(fee.trigger_order_fee_p)
}

fn pair_close_fee(
    deps: &Deps,
    pair_index: u64,
) -> Result<Decimal, ContractError> {
    let pair = PAIRS.load(deps.storage, pair_index)?;
    let fee = FEES.load(deps.storage, pair.fee_index)?;

    Ok(fee.close_fee_p)
}
