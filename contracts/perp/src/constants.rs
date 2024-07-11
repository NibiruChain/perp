use cosmwasm_std::Decimal;

pub const MAX_SL_P: Decimal = Decimal::from_ratio(75_u16, 100_u16);
pub const MAX_PNL_P: Decimal = Decimal::from_ratio(900_u16, 100_u16);
pub const LIQ_THRESHOLD_P: Decimal = Decimal::from_ratio(90_u16, 100_u16);
pub const MAX_OPEN_NEGATIVE_PNL_P: Decimal =
    Decimal::from_ratio(40_u16, 100_u16);
