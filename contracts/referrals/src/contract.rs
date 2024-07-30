use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};

use crate::{
    error::ContractError,
    state::{
        add_admin, remove_admin, Tier, BASIS_POINTS, CODE_OWNERS,
        REFERRER_DISCOUNT_SHARES, REFERRER_TIERS, TIERS, TRADER_REFERRAL_CODES,
    },
    utils::check_admin,
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct ReferralInstantiateMsg {}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: ReferralInstantiateMsg,
) -> Result<Response, ContractError> {
    add_admin(deps.branch(), info.sender)?;

    Ok(Response::default())
}

#[cw_serde]
pub enum ReferralsExecuteMsg {
    SetTier {
        tier_id: Uint128,
        total_rebate: Uint128,
        discount_share: Uint128,
    },
    SetReferrerTier {
        referrer: String,
        tier_id: Uint128,
    },
    SetReferrerDiscountShare {
        referrer: String,
        discount_share: Uint128,
    },
    SetTraderReferralCode {
        account: String,
        code: String,
    },
    SetTraderReferralCodeByUser {
        code: String,
    },
    SetCodeOwner {
        code: String,
        owner: String,
    },
    AddAdmin {
        account: String,
    },
    RemoveAdmin {
        account: String,
    },
    RegisterCode {
        code: String,
    },
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ReferralsExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ReferralsExecuteMsg::AddAdmin { account } => {
            execute_add_admin(deps, env, info, account)
        }
        ReferralsExecuteMsg::RemoveAdmin { account } => {
            execute_remove_admin(deps, env, info, account)
        }
        ReferralsExecuteMsg::SetTier {
            tier_id,
            total_rebate,
            discount_share,
        } => execute_set_tier(
            deps,
            env,
            info,
            tier_id,
            total_rebate,
            discount_share,
        ),
        ReferralsExecuteMsg::SetReferrerTier { referrer, tier_id } => {
            execute_set_referrer_tier(deps, env, info, referrer, tier_id)
        }
        ReferralsExecuteMsg::SetReferrerDiscountShare {
            referrer,
            discount_share,
        } => execute_set_referrer_discount_share(
            deps,
            env,
            info,
            referrer,
            discount_share,
        ),
        ReferralsExecuteMsg::SetTraderReferralCode { account, code } => {
            execute_set_trader_referral_code(deps, env, info, account, code)
        }
        ReferralsExecuteMsg::SetTraderReferralCodeByUser { code } => {
            execute_set_trader_referral_code_by_user(deps, env, info, code)
        }
        ReferralsExecuteMsg::SetCodeOwner { code, owner } => {
            execute_set_code_owner(deps, env, info, code, owner)
        }
        ReferralsExecuteMsg::RegisterCode { code } => {
            execute_register_code(deps, env, info, code)
        }
    }
}

pub fn execute_register_code(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code: String,
) -> Result<Response, ContractError> {
    let account = info.sender;

    if code.as_bytes() == [0u8; 32] {
        return Err(ContractError::InvalidCode {});
    }

    if CODE_OWNERS.has(deps.storage, code.as_bytes().to_vec()) {
        return Err(ContractError::CodeAlreadyClaimed {});
    }

    CODE_OWNERS.save(deps.storage, code.as_bytes().to_vec(), &account)?;

    if !REFERRER_TIERS.has(deps.storage, account.clone()) {
        if TIERS.has(deps.storage, 1) {
            REFERRER_TIERS.save(
                deps.storage,
                account.clone(),
                &Uint128::one(),
            )?;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "register_code")
        .add_attribute("account", account.to_string())
        .add_attribute("code", code))
}

pub fn execute_add_admin(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account: String,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), env, info)?;
    let account = deps.api.addr_validate(&account)?;
    add_admin(deps.branch(), account)?;
    Ok(Response::default())
}

pub fn execute_remove_admin(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    account: String,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), env, info)?;
    let account = deps.api.addr_validate(&account)?;
    remove_admin(deps.branch(), account);
    Ok(Response::default())
}

pub fn execute_set_tier(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tier_id: Uint128,
    total_rebate: Uint128,
    discount_share: Uint128,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), _env, info)?;

    // Validate total_rebate and discount_share are within bounds
    if total_rebate.gt(&BASIS_POINTS) {
        return Err(ContractError::InvalidTotalRebate {});
    }
    if discount_share.gt(&BASIS_POINTS) {
        return Err(ContractError::InvalidDiscountShare {});
    }

    let tier = Tier {
        total_rebate,
        discount_share,
    };

    TIERS.save(deps.storage, tier_id.u128(), &tier)?;

    Ok(Response::new()
        .add_attribute("action", "set_tier")
        .add_attribute("tier_id", tier_id.to_string())
        .add_attribute("total_rebate", total_rebate.to_string())
        .add_attribute("discount_share", discount_share.to_string()))
}

pub fn execute_set_referrer_tier(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    referrer: String,
    tier_id: Uint128,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), _env, info)?;

    let referrer = deps.api.addr_validate(&referrer)?;

    REFERRER_TIERS.save(deps.storage, referrer.clone(), &tier_id)?;

    Ok(Response::new()
        .add_attribute("action", "set_referrer_tier")
        .add_attribute("referrer", referrer.to_string())
        .add_attribute("tier_id", tier_id.to_string()))
}

pub fn execute_set_referrer_discount_share(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    referrer: String,
    discount_share: Uint128,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), _env, info)?;

    let referrer = deps.api.addr_validate(&referrer)?;
    let discount_share = Uint128::from(discount_share);

    REFERRER_DISCOUNT_SHARES.save(
        deps.storage,
        referrer.clone(),
        &discount_share,
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_referrer_discount_share")
        .add_attribute("referrer", referrer.to_string())
        .add_attribute("discount_share", discount_share.to_string()))
}

pub fn execute_set_trader_referral_code(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    account: String,
    code: String,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), _env, info)?;

    let account = deps.api.addr_validate(&account)?;

    TRADER_REFERRAL_CODES.save(
        deps.storage,
        account.clone(),
        &code.as_bytes().to_vec(),
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_trader_referral_code")
        .add_attribute("account", account.to_string())
        .add_attribute("code", code))
}

pub fn execute_set_trader_referral_code_by_user(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code: String,
) -> Result<Response, ContractError> {
    let account = info.sender;

    TRADER_REFERRAL_CODES.save(
        deps.storage,
        account.clone(),
        &code.as_bytes().to_vec(),
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_trader_referral_code")
        .add_attribute("account", account.to_string())
        .add_attribute("code", code))
}

pub fn execute_set_code_owner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    code: String,
    owner: String,
) -> Result<Response, ContractError> {
    check_admin(deps.as_ref(), _env, info)?;

    let owner = deps.api.addr_validate(&owner)?;

    CODE_OWNERS.save(deps.storage, code.as_bytes().to_vec(), &owner)?;

    Ok(Response::new()
        .add_attribute("action", "set_code_owner")
        .add_attribute("code", code)
        .add_attribute("owner", owner.to_string()))
}
