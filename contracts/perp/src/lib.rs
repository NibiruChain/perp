pub mod borrowing;
pub mod constants;
pub mod fees;
pub mod msgs;
pub mod pairs;
pub mod price_impact;
pub mod query;
pub mod trade;
pub mod trading;

// #[cfg(not(feature = "library"))]
// When imported with the "library" feature, contract.rs will not be compiled.
// This prevents errors related to entry the smart contract's entrypoints,
// enabling its use as a library.
pub mod contract;
pub mod error;
pub mod events;
