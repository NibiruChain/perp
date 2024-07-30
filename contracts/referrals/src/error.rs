use cosmwasm_std::StdError;
use nibiru_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("serde_json error: {0}")]
    SerdeJson(String),

    #[error("invalid code")]
    InvalidCode,

    #[error("code already claimed")]
    CodeAlreadyClaimed,

    #[error("invalid total")]
    InvalidTotalRebate,

    #[error("invalid discount share")]
    InvalidDiscountShare,

    #[error("unauthorized")]
    Unauthorized,
}

impl From<serde_json::Error> for ContractError {
    fn from(err: serde_json::Error) -> Self {
        ContractError::SerdeJson(err.to_string())
    }
}

impl From<OwnershipError> for ContractError {
    fn from(err: OwnershipError) -> Self {
        ContractError::SerdeJson(err.to_string())
    }
}
