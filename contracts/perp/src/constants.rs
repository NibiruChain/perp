use cosmwasm_std::Decimal;

pub const MAX_SL_P: Decimal = Decimal::percent(75);
pub const MAX_PNL_P: Decimal = Decimal::percent(900);
pub const LIQ_THRESHOLD_P: Decimal = Decimal::percent(90);
pub const MAX_OPEN_NEGATIVE_PNL_P: Decimal = Decimal::percent(40);
pub const GOV_PRICE_COLLATERAL_INDEX: u64 = 0;
