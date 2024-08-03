use cosmwasm_std::{Binary, Deps, Env};

use crate::{error::ContractError, msgs::QueryMsg};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(
    _deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::HasOpenLimitOrder {
            address: _String,
            pair_index: _,
            index: _u64,
        } => {
            todo!()
        }
    }
}
