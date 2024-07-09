use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Decimal256, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult,
};
use cw_storage_plus::Item;
use nibiru_ownable::{ownable_execute, ownable_query, OwnershipError};

#[cw_serde]
pub struct Price {
    pub last_update_block: u64,
    pub price: Decimal256,
}

pub const DENOMS: Item<HashMap<u64, String>> = Item::new("denoms");
pub const PRICES: Item<HashMap<u64, Price>> = Item::new("prices");

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct OracleInstantiateMsg {
    pub owner: Option<String>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: OracleInstantiateMsg,
) -> Result<Response, OwnershipError> {
    nibiru_ownable::initialize_owner(deps.storage, msg.owner.as_deref())?;
    Ok(Response::new())
}

#[ownable_execute]
#[cw_serde]
pub enum OraclesExecuteMsg {
    SetPrice { pair_index: u64, price: Decimal256 },
    SetDenom { pair_index: u64, denom: String },
    DeleteDenom { pair_index: u64 },
}

#[ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum OracleQueryMsg {
    // Retrieve the price of the given pair
    #[returns(Decimal256)]
    GetPrice { pair_index: u64 },

    // Retrieve the denomination of the given pair
    #[returns(String)]
    GetDenom { pair_index: u64 },

    // Retrieve all denominations
    #[returns(Vec<String>)]
    GetDenoms {},
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: OraclesExecuteMsg,
) -> Result<Response, OwnershipError> {
    match msg {
        OraclesExecuteMsg::SetPrice { pair_index, price } => {
            execute_set_price(deps, env, info, pair_index, price)
        }
        OraclesExecuteMsg::SetDenom { pair_index, denom } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            let mut denoms = DENOMS.load(deps.storage)?;
            denoms.insert(pair_index, denom);
            DENOMS.save(deps.storage, &denoms)?;
            Ok(Response::default())
        }
        OraclesExecuteMsg::DeleteDenom { pair_index } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            let mut denoms = DENOMS.load(deps.storage)?;
            denoms.remove(&pair_index);
            DENOMS.save(deps.storage, &denoms)?;
            Ok(Response::default())
        }
        OraclesExecuteMsg::UpdateOwnership(action) => {
            execute_update_ownership(deps, env, info, action)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: OracleQueryMsg) -> StdResult<Binary> {
    match msg {
        OracleQueryMsg::GetPrice { pair_index } => get_price(deps, pair_index),
        OracleQueryMsg::Ownership {} => Ok(to_json_binary(
            &nibiru_ownable::get_ownership(deps.storage)?,
        )?),
        OracleQueryMsg::GetDenom { pair_index } => get_denom(deps, pair_index),
        OracleQueryMsg::GetDenoms {} => get_denoms(deps),
    }
}

fn get_denom(deps: Deps, pair_index: u64) -> StdResult<Binary> {
    let denoms = DENOMS.load(deps.storage)?;
    to_json_binary(&denoms.get(&pair_index))
}

fn get_denoms(deps: Deps) -> StdResult<Binary> {
    let denoms = DENOMS.load(deps.storage)?;
    to_json_binary(&denoms)
}

fn get_price(deps: Deps, pair_index: u64) -> StdResult<Binary> {
    let prices = PRICES.load(deps.storage)?;
    let price = prices.get(&pair_index).cloned();
    to_json_binary(&price)
}

fn execute_set_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_index: u64,
    price: Decimal256,
) -> Result<Response, OwnershipError> {
    nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;

    let mut prices = PRICES.load(deps.storage)?;
    prices.insert(
        pair_index,
        Price {
            price,
            last_update_block: env.block.height,
        },
    );
    PRICES.save(deps.storage, &prices)?;

    Ok(Response::default())
}

pub fn execute_update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: nibiru_ownable::Action,
) -> Result<Response, OwnershipError> {
    let ownership = nibiru_ownable::update_ownership(
        deps,
        &env.block,
        info.sender.as_str(),
        action,
    )
    .unwrap();
    Ok(Response::new().add_attributes(ownership.into_attributes()))
}
