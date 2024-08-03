use anyhow::Result;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

use crate::{
    borrowing::state::{GROUP_OIS, PAIR_OIS},
    fees::state::{FEE_TIERS, PENDING_GOV_FEES, TRADER_DAILY_INFOS},
    msgs::AdminExecuteMsg,
    pairs::state::{
        FEES, GROUPS, ORACLE_ADDRESS, PAIRS, PAIR_CUSTOM_MAX_LEVERAGE,
        STAKING_ADDRESS,
    },
    trade::{
        cancel_open_limit_order, close_trade_market, execute_limit_order,
        open_trade, update_open_limit_order, update_sl, update_tp,
    },
};

use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msgs::{ExecuteMsg, InstantiateMsg},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(
        deps.storage,
        format!("crates.io:{CONTRACT_NAME}"),
        CONTRACT_VERSION,
    )?;

    nibiru_ownable::initialize_owner(deps.storage, msg.owner.as_deref())?;
    if let Some(oracle_address) = msg.oracle_address {
        ORACLE_ADDRESS.save(deps.storage, &Addr::unchecked(oracle_address))?;
    }
    if let Some(staking_address) = msg.staking_address {
        STAKING_ADDRESS.save(deps.storage, &Addr::unchecked(staking_address))?;
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenTrade {
            trade,
            order_type,
            spread_reduction_id: _,
            slippage_p,
            referral: _,
        } => open_trade(&mut deps, env, info, trade, order_type, slippage_p),
        ExecuteMsg::CloseTradeMarket { index } => {
            close_trade_market(&mut deps, env, info, index)
        }
        ExecuteMsg::UpdateOpenLimitOrder {
            pair_index,
            index,
            price,
            tp,
            sl,
        } => update_open_limit_order(
            deps, env, info, pair_index, index, price, tp, sl,
        ),
        ExecuteMsg::CancelOpenLimitOrder { pair_index, index } => {
            cancel_open_limit_order(deps, env, info, pair_index, index)
        }
        ExecuteMsg::UpdateTp {
            pair_index,
            index,
            new_tp,
        } => update_tp(deps, env, info, pair_index, index, new_tp),
        ExecuteMsg::UpdateSl {
            pair_index,
            index,
            new_sl,
        } => update_sl(deps, env, info, pair_index, index, new_sl),
        ExecuteMsg::ExecuteNftOrder {
            order_type,
            trader,
            pair_index,
            index,
            nft_id,
            nft_type,
        } => execute_limit_order(
            deps, env, info, order_type, trader, pair_index, index, nft_id,
            nft_type,
        ),
        ExecuteMsg::AdminMsg { msg } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            execute_admin(&mut deps, env, msg)
        }
    }
}

// todo: add event to each responses
pub(crate) fn execute_admin(
    deps: &mut DepsMut,
    _env: Env,
    msg: AdminExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        AdminExecuteMsg::SetPairs { pairs } => {
            for (index, pair) in pairs.iter() {
                PAIRS.save(deps.storage, *index, pair)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::SetGroups { groups } => {
            for (index, group) in groups.iter() {
                GROUPS.save(deps.storage, *index, group)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::SetFees { fees } => {
            for (index, fee) in fees.iter() {
                FEES.save(deps.storage, *index, fee)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::SetPairCustomMaxLeverage {
            pair_custom_max_leverage,
        } => {
            for (index, max_leverage) in pair_custom_max_leverage.iter() {
                PAIR_CUSTOM_MAX_LEVERAGE.save(
                    deps.storage,
                    *index,
                    max_leverage,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateOracleAddress { oracle_address } => {
            ORACLE_ADDRESS
                .save(deps.storage, &Addr::unchecked(oracle_address))?;
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateStakingAddress { staking_address } => {
            STAKING_ADDRESS
                .save(deps.storage, &Addr::unchecked(staking_address))?;
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateFeeTiers { fee_tiers } => {
            FEE_TIERS.save(deps.storage, &fee_tiers)?;
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdatePendingGovFees { pending_gov_fees } => {
            for (index, fee) in pending_gov_fees.iter() {
                PENDING_GOV_FEES.save(deps.storage, *index, fee)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateTraderDailyInfos { trader_daily_infos } => {
            for (index, info) in trader_daily_infos.iter() {
                TRADER_DAILY_INFOS.save(deps.storage, index.clone(), info)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateBorrowingPairs { borrowing_pairs } => {
            for (index, pair) in borrowing_pairs.iter() {
                crate::borrowing::state::PAIRS.save(
                    deps.storage,
                    *index,
                    pair,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateBorrowingPairGroups { pair_groups } => {
            for (index, group) in pair_groups.iter() {
                crate::borrowing::state::PAIR_GROUPS.save(
                    deps.storage,
                    *index,
                    group,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateBorrowingPairOis { pair_ois } => {
            for (index, oi) in pair_ois.iter() {
                PAIR_OIS.save(deps.storage, *index, oi)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateBorrowingGroups { groups } => {
            for (index, group) in groups.iter() {
                crate::borrowing::state::GROUPS.save(
                    deps.storage,
                    *index,
                    group,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateBorrowingGroupOis { group_ois } => {
            for (index, oi) in group_ois.iter() {
                GROUP_OIS.save(deps.storage, *index, oi)?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateOiWindowsSettings {
            oi_windows_settings,
        } => {
            crate::price_impact::state::OI_WINDOWS_SETTINGS
                .save(deps.storage, &oi_windows_settings)?;
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateWindows { windows } => {
            for (index, window) in windows.iter() {
                crate::price_impact::state::WINDOWS.save(
                    deps.storage,
                    *index,
                    window,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdatePairDepths { pair_depths } => {
            for (index, depth) in pair_depths.iter() {
                crate::price_impact::state::PAIR_DEPTHS.save(
                    deps.storage,
                    *index,
                    depth,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateCollaterals { collaterals } => {
            for (index, collateral) in collaterals.iter() {
                crate::trading::state::COLLATERALS.save(
                    deps.storage,
                    *index,
                    collateral,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateTrades { trades } => {
            for (index, trade) in trades.iter() {
                crate::trading::state::TRADES.save(
                    deps.storage,
                    index.clone(),
                    trade,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateTradeInfos { trade_infos } => {
            for (index, info) in trade_infos.iter() {
                crate::trading::state::TRADE_INFOS.save(
                    deps.storage,
                    index.clone(),
                    info,
                )?;
            }
            Ok(Response::new())
        }
        AdminExecuteMsg::UpdateTraderStored { trader_stored } => {
            for (index, stored) in trader_stored.iter() {
                crate::trading::state::TRADER_STORED.save(
                    deps.storage,
                    index.clone(),
                    stored,
                )?;
            }
            Ok(Response::new())
        }
    }
}
