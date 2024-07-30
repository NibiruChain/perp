use cosmwasm_std::{Deps, Env, MessageInfo};

use crate::{error::ContractError, state::is_admin};

/// Check if the sender is the admin
pub fn check_admin(
    deps: Deps,
    _env: Env,
    info: MessageInfo,
) -> Result<(), ContractError> {
    let is_admin = is_admin(deps, info.sender);

    if is_admin {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}
