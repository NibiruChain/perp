use cosmwasm_schema::{cw_serde, QueryResponses};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cw_storage_plus::Map;
use nibiru_ownable::{ownable_execute, ownable_query, OwnershipError};

#[cw_serde]
pub struct Price {
    pub last_update_block: u64,
    pub price: Decimal,
}

pub const PRICES: Map<u64, Price> = Map::new("prices");
pub const COLLATERAL_PRICES: Map<u64, Price> = Map::new("collateral_prices");

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
    SetPrice { index: u64, price: Decimal },
    SetCollateralPrice { index: u64, price: Decimal },
}

#[ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum OracleQueryMsg {
    // Retrieve the price of the given pair
    #[returns(Decimal)]
    GetPrice { index: u64 },

    #[returns(Decimal)]
    GetCollateralPrice { index: u64 },
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: OraclesExecuteMsg,
) -> Result<Response, OwnershipError> {
    match msg {
        OraclesExecuteMsg::SetPrice { index, price } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            PRICES.save(
                deps.storage,
                index,
                &Price {
                    price,
                    last_update_block: env.block.height,
                },
            )?;

            Ok(Response::default())
        }
        OraclesExecuteMsg::SetCollateralPrice { index, price } => {
            nibiru_ownable::assert_owner(deps.storage, info.sender.as_str())?;
            COLLATERAL_PRICES.save(
                deps.storage,
                index,
                &Price {
                    price,
                    last_update_block: env.block.height,
                },
            )?;

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
        OracleQueryMsg::GetPrice { index } => {
            let price = PRICES.load(deps.storage, index)?;
            to_json_binary(&price)
        }
        OracleQueryMsg::GetCollateralPrice { index } => {
            let price = COLLATERAL_PRICES.load(deps.storage, index)?;
            to_json_binary(&price)
        }
        OracleQueryMsg::Ownership {} => Ok(to_json_binary(
            &nibiru_ownable::get_ownership(deps.storage)?,
        )?),
    }
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
