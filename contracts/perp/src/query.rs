use cosmwasm_std::{Binary, Deps, Env};

use crate::{error::ContractError, msgs::QueryMsg};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::HasOpenLimitOrder {
            address: String,
            pair_index,
            index: u64,
        } => {
            todo!()
        }
    }
}
