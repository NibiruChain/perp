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

            Ok(Response::new().add_attribute("method", "SetPrice"))
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

            Ok(Response::new().add_attribute("method", "SetCollateralPrice"))
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
            let price = PRICES.load(deps.storage, index)?.price;
            to_json_binary(&price)
        }
        OracleQueryMsg::GetCollateralPrice { index } => {
            let price = COLLATERAL_PRICES.load(deps.storage, index)?;
            to_json_binary(&price.price)
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{attr, from_json, Addr, Decimal};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = OracleInstantiateMsg {
            owner: Some("owner".to_string()),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let owner =
            nibiru_ownable::get_ownership(deps.as_ref().storage).unwrap();
        assert_eq!(owner.owner, Some("owner".to_string()));
    }

    #[test]
    fn set_and_get_price() {
        let mut deps = mock_dependencies();
        let msg = OracleInstantiateMsg {
            owner: Some("owner".to_string()),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = message_info(&Addr::unchecked("owner"), &[]);
        let msg = OraclesExecuteMsg::SetPrice {
            index: 1,
            price: Decimal::percent(100),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes, vec![attr("method", "SetPrice")]);

        let msg = OracleQueryMsg::GetPrice { index: 1 };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let price: Decimal = from_json(&res).unwrap();
        assert_eq!(price, Decimal::percent(100));
    }

    #[test]
    fn set_and_get_collateral_price() {
        let mut deps = mock_dependencies();
        let msg = OracleInstantiateMsg {
            owner: Some("owner".to_string()),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = message_info(&Addr::unchecked("owner"), &[]);
        let msg = OraclesExecuteMsg::SetCollateralPrice {
            index: 1,
            price: Decimal::percent(200),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes, vec![attr("method", "SetCollateralPrice")]);

        let msg = OracleQueryMsg::GetCollateralPrice { index: 1 };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let price: Decimal = from_json(&res).unwrap();
        assert_eq!(price, Decimal::percent(200));
    }

    #[test]
    fn unauthorized_set_price() {
        let mut deps = mock_dependencies();
        let msg = OracleInstantiateMsg {
            owner: Some("owner".to_string()),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = message_info(&Addr::unchecked("not_owner"), &[]);
        let msg = OraclesExecuteMsg::SetPrice {
            index: 1,
            price: Decimal::percent(100),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            OwnershipError::NotOwner {} => {}
            _ => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn unauthorized_set_collateral_price() {
        let mut deps = mock_dependencies();
        let msg = OracleInstantiateMsg {
            owner: Some("owner".to_string()),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = message_info(&Addr::unchecked("not_owner"), &[]);
        let msg = OraclesExecuteMsg::SetCollateralPrice {
            index: 1,
            price: Decimal::percent(200),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            OwnershipError::NotOwner {} => {}
            _ => panic!("Unexpected error: {:?}", err),
        }
    }
}
