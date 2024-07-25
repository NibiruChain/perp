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
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: OracleInstantiateMsg,
) -> Result<Response, OwnershipError> {
    nibiru_ownable::initialize_owner(deps.storage, msg.owner.as_deref())?;
    Ok(Response::new())
}

#[ownable_execute]
#[cw_serde]
pub enum OraclesExecuteMsg {
    SetPrice {
        oracle_index: u64,
        price: Decimal256,
    },
    SetDenom {
        oracle_index: u64,
        denom: String,
    },
    DeleteDenom {
        oracle_index: u64,
    },
}

#[ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum OracleQueryMsg {
    // Retrieve the price of the given pair
    #[returns(Decimal256)]
    GetPrice { oracle_index: u64 },

    // Retrieve the denomination of the given pair
    #[returns(String)]
    GetDenom { oracle_index: u64 },

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
        OraclesExecuteMsg::SetPrice {
            oracle_index,
            price,
        } => execute_set_price(deps, env, info, oracle_index, price),
        OraclesExecuteMsg::SetDenom {
            oracle_index,
            denom,
        } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            let mut denoms = DENOMS.load(deps.storage)?;
            denoms.insert(oracle_index, denom);
            DENOMS.save(deps.storage, &denoms)?;
            Ok(Response::default())
        }
        OraclesExecuteMsg::DeleteDenom { oracle_index } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            let mut denoms = DENOMS.load(deps.storage)?;
            denoms.remove(&oracle_index);
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
        OracleQueryMsg::GetPrice { oracle_index } => {
            get_price(deps, oracle_index)
        }
        OracleQueryMsg::Ownership {} => Ok(to_json_binary(
            &nibiru_ownable::get_ownership(deps.storage)?,
        )?),
        OracleQueryMsg::GetDenom { oracle_index } => {
            get_denom(deps, oracle_index)
        }
        OracleQueryMsg::GetDenoms {} => get_denoms(deps),
    }
}

fn get_denom(deps: Deps, oracle_index: u64) -> StdResult<Binary> {
    let denoms = DENOMS.load(deps.storage)?;
    to_json_binary(&denoms.get(&oracle_index))
}

fn get_denoms(deps: Deps) -> StdResult<Binary> {
    let denoms = DENOMS.load(deps.storage)?;
    to_json_binary(&denoms)
}

fn get_price(deps: Deps, oracle_index: u64) -> StdResult<Binary> {
    let prices = PRICES.load(deps.storage)?;
    let price = prices.get(&oracle_index).cloned();
    to_json_binary(&price)
}

fn execute_set_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    oracle_index: u64,
    price: Decimal256,
) -> Result<Response, OwnershipError> {
    nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;

    let mut prices = PRICES.load(deps.storage)?;
    prices.insert(
        oracle_index,
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
