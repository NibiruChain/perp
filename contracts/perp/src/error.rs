use cosmwasm_std::{
    DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
};
use nibiru_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("serde_json error: {0}")]
    SerdeJson(String),

    #[error("not implemented")]
    NotImplemented,

    #[error("operations are currently halted")]
    OperationsHalted,

    #[error("invalid leverage value")]
    InvalidLeverage,

    #[error("invalid position size")]
    InvalidPositionSize,

    #[error("invalid limit order type")]
    InvalidLimitOrderType,

    #[error("invalid tp or sl value")]
    InvalidTpSl,

    #[error("maximum trades per pair reached")]
    MaxTradesPerPair,

    #[error("maximum pending orders reached")]
    MaxPendingOrders,

    #[error("order already being closed")]
    AlreadyBeingClosed,

    #[error("price impact too high")]
    PriceImpactTooHigh,

    #[error("outside of exposure limits")]
    OutsideExposureLimits,

    #[error("no corresponding NFT for spread reduction")]
    NoCorrespondingNftSpreadReduction,

    #[error("trade does not exist")]
    TradeDoesNotExist,

    #[error("limit order does not exist")]
    LimitOrderDoesNotExist,

    #[error("market order timed out")]
    MarketOrderTimeout,

    #[error("SL value is too big")]
    SlTooBig,

    #[error("limit order timelock not expired")]
    LimitOrderTimelock,

    #[error("NFT success timelock not expired")]
    NftSuccessTimelock,

    #[error("invalid referral address")]
    InvalidReferral,

    #[error("trade was not found")]
    TradeNotFound,

    #[error("trade invalid")]
    TradeInvalid,

    #[error("inssuficient collateral")]
    InsufficientCollateral,
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

impl From<OverflowError> for ContractError {
    fn from(err: OverflowError) -> Self {
        ContractError::SerdeJson(err.to_string())
    }
}

impl From<DecimalRangeExceeded> for ContractError {
    fn from(err: DecimalRangeExceeded) -> Self {
        ContractError::SerdeJson(err.to_string())
    }
}

impl From<DivideByZeroError> for ContractError {
    fn from(err: DivideByZeroError) -> Self {
        ContractError::SerdeJson(err.to_string())
    }
}
