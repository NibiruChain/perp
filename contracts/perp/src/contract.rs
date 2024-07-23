use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::trade::{
    cancel_open_limit_order, close_trade_market, execute_limit_order,
    open_trade, update_open_limit_order, update_sl, update_tp,
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
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenTrade {
            trade,
            order_type,
            spread_reduction_id,
            slippage_p,
            referral,
        } => open_trade(deps, env, info, trade, order_type, slippage_p),
        ExecuteMsg::CloseTradeMarket { pair_index, index } => {
            close_trade_market(deps, env, info, pair_index, index)
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
            Ok(Response::default())
        }
    }
}
