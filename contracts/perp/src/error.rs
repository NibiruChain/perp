use cosmwasm_std::StdError;
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

    #[error("price impact too high")]
    PriceImpactTooHigh,

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
}

impl From<serde_json::Error> for ContractError {
    fn from(err: serde_json::Error) -> Self {
        ContractError::SerdeJson(err.to_string())
    }
}
