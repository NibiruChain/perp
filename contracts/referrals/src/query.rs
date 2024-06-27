use cosmwasm_schema::{cw_serde, QueryResponses};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, StdError, StdResult, Uint128};

use crate::state::{
    Tier, CODE_OWNERS, REFERRER_DISCOUNT_SHARES, REFERRER_TIERS, TIERS, TRADER_REFERRAL_CODES,
};

#[cw_serde]
#[derive(QueryResponses)]
pub enum ReferralsQueryMsg {
    // Retrieves the owners of given codes.
    #[returns(Vec<String>)]
    GetCodesOwner {
        codes: Vec<String>,
    },

    // Returns the owner of a specific code.
    #[returns(String)]
    GetCodeOwner {
        code: String,
    },

    // Returns the discount share of a referrer.
    #[returns(Uint128)]
    ReferrerDiscountShares {
        account: String,
    },

    // Returns the tier of a referrer.
    #[returns(Uint128)]
    ReferrerTiers {
        account: String,
    },

    // Returns the referrer associated with an account.
    #[returns(String)]
    GetTraderReferralInfo {
        account: String,
    },

    #[returns(Tier)]
    GetTier {
        tier_id: Uint128,
    },
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: ReferralsQueryMsg) -> StdResult<Binary> {
    match msg {
        ReferralsQueryMsg::GetCodesOwner {
            codes,
        } => get_codes_owner(deps, codes),
        ReferralsQueryMsg::GetCodeOwner {
            code,
        } => get_code_owner(deps, &code),
        ReferralsQueryMsg::ReferrerDiscountShares {
            account,
        } => get_referrer_discount_shares(deps, &account),
        ReferralsQueryMsg::ReferrerTiers {
            account,
        } => get_referrer_tiers(deps, &account),
        ReferralsQueryMsg::GetTraderReferralInfo {
            account,
        } => get_trader_referral_info(deps, &account),
        ReferralsQueryMsg::GetTier {
            tier_id,
        } => get_tier(deps, tier_id),
    }
}

fn get_tier(deps: Deps<'_>, tier_id: Uint128) -> Result<Binary, StdError> {
    let tier: Tier = TIERS.load(deps.storage, tier_id.u128())?;
    to_json_binary(&tier)
}

fn get_trader_referral_info(deps: Deps<'_>, account: &str) -> Result<Binary, StdError> {
    let address = deps.api.addr_validate(account)?;

    if let Some(code) = TRADER_REFERRAL_CODES.may_load(deps.storage, address)? {
        if let Some(owner) = CODE_OWNERS.may_load(deps.storage, code.clone())? {
            to_json_binary(&owner)
        } else {
            Err(StdError::generic_err("Code owner not found"))
        }
    } else {
        Err(StdError::generic_err("Referral code not found"))
    }
}

fn get_referrer_tiers(deps: Deps<'_>, account: &str) -> Result<Binary, StdError> {
    let tier = REFERRER_TIERS.may_load(deps.storage, deps.api.addr_validate(account)?)?;
    match tier {
        Some(tier) => to_json_binary(&tier),
        None => Err(StdError::generic_err("Tier not found")),
    }
}

fn get_referrer_discount_shares(deps: Deps<'_>, account: &str) -> Result<Binary, StdError> {
    let discount_share =
        REFERRER_DISCOUNT_SHARES.may_load(deps.storage, deps.api.addr_validate(account)?)?;
    match discount_share {
        Some(discount_share) => to_json_binary(&discount_share),
        None => to_json_binary(&Uint128::zero()),
    }
}

fn get_code_owner(deps: Deps<'_>, code: &str) -> Result<Binary, StdError> {
    let owner = load_code_owner(deps, code)?;
    to_json_binary(&owner)
}

fn get_codes_owner(deps: Deps<'_>, codes: Vec<String>) -> Result<Binary, StdError> {
    let owners: Vec<String> =
        codes.into_iter().map(|code| load_code_owner(deps, &code).unwrap_or_default()).collect();
    to_json_binary(&owners)
}

fn load_code_owner(deps: Deps<'_>, code: &str) -> Result<String, StdError> {
    match CODE_OWNERS.may_load(deps.storage, code.as_bytes().to_vec()) {
        Ok(Some(owner)) => Ok(owner.to_string()),
        Ok(None) => Ok("".to_string()),
        Err(e) => Err(e),
    }
}
