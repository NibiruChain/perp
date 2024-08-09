use cosmwasm_std::{Decimal, Deps, SignedDecimal, Storage, Uint128};
use oracle::contract::OracleQueryMsg;

use crate::{
    borrowing::state::{GROUP_OIS, PAIR_OIS},
    constants::{MAX_PNL_P, MAX_SL_P},
    error::ContractError,
    pairs::state::{FEES, ORACLE_ADDRESS, PAIRS},
    utils::{dec_to_sdec, u128_to_dec},
};

pub(crate) fn get_market_execution_price(
    price: Decimal,
    spread_p: Decimal,
    long: bool,
) -> Decimal {
    let price_diff = price.checked_mul(spread_p).unwrap();
    if long {
        price.checked_add(price_diff).unwrap()
    } else {
        price.checked_sub(price_diff).unwrap()
    }
}

pub(crate) fn get_position_size_collateral(
    collateral_amount: Uint128,
    leverage: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(leverage.checked_mul(collateral_amount)?)
}

pub(crate) fn within_exposure_limits(
    storage: &dyn Storage,
    pair_index: u64,
    group_index: u64,
    collateral_index: u64,
    long: bool,
    position_size_collateral: Uint128,
) -> Result<(), ContractError> {
    let pair_ois = PAIR_OIS.load(storage, (collateral_index, pair_index))?;
    let group_ois = GROUP_OIS.load(storage, (collateral_index, group_index))?;

    let pair_oi_collateral = if long { pair_ois.long } else { pair_ois.short };
    let group_oi_collateral = if long {
        group_ois.long
    } else {
        group_ois.short
    };

    let within_group_limit = position_size_collateral + group_oi_collateral
        <= group_ois.max
        || group_ois.max.is_zero();

    let within_pair_limit =
        position_size_collateral + pair_oi_collateral <= pair_ois.max;

    if within_pair_limit && within_group_limit {
        return Ok(());
    }
    Err(ContractError::ExposureLimitReached)
}

pub(crate) fn get_position_size_collateral_basis(
    deps: &Deps,
    collateral_index: &u64,
    pair_index: &u64,
    position_size_collateral: Uint128,
) -> Result<Uint128, ContractError> {
    let pair = PAIRS.load(deps.storage, *pair_index)?;
    let min_fee = FEES.load(deps.storage, pair.fee_index)?.get_min_fee_usd()?;
    let collateral_price = get_collateral_price_usd(deps, *collateral_index)?;

    let min_fee_collateral = u128_to_dec(min_fee)?
        .checked_div(collateral_price)?
        .to_uint_floor();

    Ok(Uint128::max(position_size_collateral, min_fee_collateral))
}

pub fn get_collateral_price_usd(
    deps: &Deps,
    collateral_index: u64,
) -> Result<Decimal, ContractError> {
    get_collateral_price(deps, &collateral_index)
}

pub fn get_collateral_price(
    deps: &Deps,
    oracle_index: &u64,
) -> Result<Decimal, ContractError> {
    Ok(deps.querier.query_wasm_smart::<Decimal>(
        ORACLE_ADDRESS.load(deps.storage)?.to_string(),
        &OracleQueryMsg::GetCollateralPrice {
            index: *oracle_index,
        },
    )?)
}

pub(crate) fn limit_tp_distance(
    open_price: Decimal,
    leverage: Uint128,
    tp: Decimal,
    long: bool,
) -> Result<Decimal, ContractError> {
    if tp.is_zero()
        || get_pnl_percent(open_price, tp, long, leverage)?
            == dec_to_sdec(MAX_PNL_P)?
    {
        let open_price = open_price;
        let tp_diff =
            (open_price * MAX_PNL_P).checked_div(u128_to_dec(leverage)?)?;
        let new_tp = if long {
            open_price + tp_diff
        } else if tp_diff <= open_price {
            open_price - tp_diff
        } else {
            Decimal::zero()
        };
        return Ok(new_tp);
    }
    Ok(tp)
}

pub(crate) fn limit_sl_distance(
    open_price: Decimal,
    leverage: Uint128,
    sl: Decimal,
    long: bool,
) -> Result<Decimal, ContractError> {
    if !sl.is_zero()
        && get_pnl_percent(open_price, sl, long, leverage)?
            < dec_to_sdec(MAX_SL_P)? * SignedDecimal::percent(-100)
    {
        let sl_diff =
            (open_price * MAX_SL_P).checked_div(u128_to_dec(leverage)?)?;
        let new_sl = if long {
            open_price.checked_sub(sl_diff)?
        } else {
            open_price.checked_add(sl_diff)?
        };
        Ok(new_sl)
    } else {
        Ok(sl)
    }
}

/// Compute the PnL percentage of a trade
/// Bounded by -100% and MAX_PNL_P
pub(crate) fn get_pnl_percent(
    open_price: Decimal,
    current_price: Decimal,
    long: bool,
    leverage: Uint128,
) -> Result<SignedDecimal, ContractError> {
    if !open_price.is_zero() {
        let current_price = SignedDecimal::try_from(current_price)?;
        let open_price = SignedDecimal::try_from(open_price)?;

        let pnl = if long {
            current_price.checked_sub(open_price)?
        } else {
            open_price.checked_sub(current_price)?
        };

        let pnl_percent = pnl.checked_div(open_price)?;
        let leverage = SignedDecimal::try_from(u128_to_dec(leverage)?)?;
        let pnl_percent = pnl_percent.checked_mul(leverage)?;

        return Ok(SignedDecimal::max(
            SignedDecimal::percent(-100),
            SignedDecimal::min(pnl_percent, dec_to_sdec(MAX_PNL_P)?),
        ));
    }
    Ok(SignedDecimal::zero())
}

#[cfg(test)]
mod tests {
    use crate::borrowing::state::OpenInterest;

    use super::*;
    use cosmwasm_std::{
        testing::{mock_dependencies, MockStorage},
        Decimal, Uint128,
    };

    #[test]
    fn test_market_execution_price_long_position() {
        // Test when long is true (buying)
        let price = Decimal::from_ratio(Uint128::new(100), Uint128::new(1)); // 100.0
        let spread_p = Decimal::from_ratio(Uint128::new(5), Uint128::new(100)); // 0.05 (5%)
        let long = true;

        let expected_price =
            Decimal::from_ratio(Uint128::new(105), Uint128::new(1)); // 105.0
        let result = get_market_execution_price(price, spread_p, long);

        assert_eq!(result, expected_price);
    }

    #[test]
    fn test_market_execution_price_short_position() {
        // Test when long is false (selling)
        let price = Decimal::from_ratio(Uint128::new(100), Uint128::new(1)); // 100.0
        let spread_p = Decimal::from_ratio(Uint128::new(5), Uint128::new(100)); // 0.05 (5%)
        let long = false;

        let expected_price =
            Decimal::from_ratio(Uint128::new(95), Uint128::new(1)); // 95.0
        let result = get_market_execution_price(price, spread_p, long);

        assert_eq!(result, expected_price);
    }

    #[test]
    fn test_market_execution_price_zero_spread() {
        // Test when the spread is zero
        let price = Decimal::from_ratio(Uint128::new(100), Uint128::new(1)); // 100.0
        let spread_p = Decimal::zero();
        let long = true;

        let expected_price =
            Decimal::from_ratio(Uint128::new(100), Uint128::new(1)); // 100.0
        let result = get_market_execution_price(price, spread_p, long);

        assert_eq!(result, expected_price);
    }

    #[test]
    fn test_market_execution_price_large_spread() {
        // Test when the spread is large (e.g., 50%)
        let price = Decimal::from_ratio(Uint128::new(100), Uint128::new(1)); // 100.0
        let spread_p = Decimal::from_ratio(Uint128::new(50), Uint128::new(100)); // 0.50 (50%)
        let long = false;

        let expected_price =
            Decimal::from_ratio(Uint128::new(50), Uint128::new(1)); // 50.0
        let result = get_market_execution_price(price, spread_p, long);

        assert_eq!(result, expected_price);
    }

    #[test]
    fn test_position_size_collateral_normal_case() {
        // Test with normal values
        let collateral_amount = Uint128::new(100); // 100 units of collateral
        let leverage = Uint128::new(5); // 5x leverage

        let expected_size = Uint128::new(500); // 100 * 5 = 500
        let result = get_position_size_collateral(collateral_amount, leverage);

        assert_eq!(result.unwrap(), expected_size);
    }

    struct ExposureLimitTestCase {
        description: &'static str,
        pair_oi: OpenInterest,
        group_oi: OpenInterest,
        position_size_collateral: Uint128,
        long: bool,
        expected_result: Result<(), ContractError>,
    }

    fn setup_storage_with_oi(
        storage: &mut MockStorage,
        pair_oi: OpenInterest,
        group_oi: OpenInterest,
    ) {
        PAIR_OIS.save(storage, (1, 1), &pair_oi).unwrap();
        GROUP_OIS.save(storage, (1, 1), &group_oi).unwrap();
    }

    #[test]
    fn test_within_exposure_limits_cases() {
        let deps = mock_dependencies();
        let mut storage = deps.storage;

        let test_cases = vec![
            ExposureLimitTestCase {
                description: "Within limits",
                pair_oi: OpenInterest {
                    long: Uint128::new(100),
                    short: Uint128::new(50),
                    max: Uint128::new(200),
                },
                group_oi: OpenInterest {
                    long: Uint128::new(300),
                    short: Uint128::new(150),
                    max: Uint128::new(500),
                },
                position_size_collateral: Uint128::new(50),
                long: true,
                expected_result: Ok(()),
            },
            ExposureLimitTestCase {
                description: "Exceed pair limit",
                pair_oi: OpenInterest {
                    long: Uint128::new(101),
                    short: Uint128::new(1),
                    max: Uint128::new(200),
                },
                group_oi: OpenInterest {
                    long: Uint128::new(1),
                    short: Uint128::new(2),
                    max: Uint128::new(200),
                },
                position_size_collateral: Uint128::new(100),
                long: true,
                expected_result: Err(ContractError::ExposureLimitReached),
            },
            ExposureLimitTestCase {
                description: "Exceed group limit",
                pair_oi: OpenInterest {
                    long: Uint128::new(100),
                    short: Uint128::new(50),
                    max: Uint128::new(200),
                },
                group_oi: OpenInterest {
                    long: Uint128::new(400),
                    short: Uint128::new(150),
                    max: Uint128::new(500),
                },
                position_size_collateral: Uint128::new(150),
                long: true,
                expected_result: Err(ContractError::ExposureLimitReached),
            },
            ExposureLimitTestCase {
                description: "Exceed both limits",
                pair_oi: OpenInterest {
                    long: Uint128::new(150),
                    short: Uint128::new(50),
                    max: Uint128::new(200),
                },
                group_oi: OpenInterest {
                    long: Uint128::new(450),
                    short: Uint128::new(150),
                    max: Uint128::new(500),
                },
                position_size_collateral: Uint128::new(100),
                long: true,
                expected_result: Err(ContractError::ExposureLimitReached),
            },
        ];

        for test in test_cases {
            // Setup storage for the test case
            setup_storage_with_oi(
                &mut storage,
                test.pair_oi.clone(),
                test.group_oi.clone(),
            );

            // Execute the test
            let result = within_exposure_limits(
                &storage,
                1, // pair_index
                1, // group_index
                1, // collateral_index
                test.long,
                test.position_size_collateral,
            );

            // Assert the result matches the expected outcome
            assert_eq!(
                result, test.expected_result,
                "Failed test: {}",
                test.description
            );
        }
    }

    struct TpDistanceTestCase {
        description: &'static str,
        open_price: &'static str,
        leverage: Uint128,
        tp: &'static str,
        long: bool,
        expected_result: Result<Decimal, ContractError>,
    }

    #[test]
    fn test_limit_tp_distance_cases() {
        let test_cases = vec![
            TpDistanceTestCase {
                description: "TP within limits for long position",
                open_price: "100",
                leverage: Uint128::new(10),
                tp: "120",
                long: true,
                expected_result: Ok("120".parse::<Decimal>().unwrap()),
            },
            TpDistanceTestCase {
                description: "TP exceeds limit for long position",
                open_price: "100",
                leverage: Uint128::new(10),
                tp: "200",
                long: true,
                expected_result: Ok("190".parse::<Decimal>().unwrap()),
            },
            TpDistanceTestCase {
                description: "TP within limits for short position",
                open_price: "100",
                leverage: Uint128::new(2),
                tp: "80",
                long: false,
                expected_result: Ok("80".parse::<Decimal>().unwrap()),
            },
            TpDistanceTestCase {
                description: "TP exceeds limit for short position",
                open_price: "100",
                leverage: Uint128::new(10),
                tp: "5",
                long: false,
                expected_result: Ok("10".parse::<Decimal>().unwrap()),
            },
        ];

        for test in test_cases {
            let result = limit_tp_distance(
                test.open_price.parse::<Decimal>().unwrap(),
                test.leverage,
                test.tp.parse::<Decimal>().unwrap(),
                test.long,
            );
            assert_eq!(
                result, test.expected_result,
                "Failed test: {}",
                test.description
            );
        }
    }

    struct SlDistanceTestCase {
        description: &'static str,
        open_price: &'static str,
        leverage: Uint128,
        sl: &'static str,
        long: bool,
        expected_result: Result<Decimal, ContractError>,
    }
    #[test]
    fn test_limit_sl_distance_cases() {
        let test_cases = vec![
            SlDistanceTestCase {
                description: "SL within limits for long position",
                open_price: "100",
                leverage: Uint128::new(1),
                sl: "80",
                long: true,
                expected_result: Ok("80".parse::<Decimal>().unwrap()),
            },
            SlDistanceTestCase {
                description: "SL exceeds limit for long position",
                open_price: "100",
                leverage: Uint128::new(10),
                sl: "80",
                long: true,
                expected_result: Ok("92.5".parse::<Decimal>().unwrap()), // max sl_p = 75%
            },
            SlDistanceTestCase {
                description: "SL within limits for short position",
                open_price: "100",
                leverage: Uint128::new(10),
                sl: "105",
                long: false,
                expected_result: Ok("105".parse::<Decimal>().unwrap()),
            },
            SlDistanceTestCase {
                description: "SL exceeds limit for short position",
                open_price: "100",
                leverage: Uint128::new(10),
                sl: "110",
                long: false,
                expected_result: Ok("107.5".parse::<Decimal>().unwrap()),
            },
        ];

        for test in test_cases {
            let result = limit_sl_distance(
                test.open_price.parse::<Decimal>().unwrap(),
                test.leverage,
                test.sl.parse::<Decimal>().unwrap(),
                test.long,
            );
            assert_eq!(
                result, test.expected_result,
                "Failed test: {}",
                test.description
            );
        }
    }

    struct PnlPercentTestCase {
        description: &'static str,
        open_price: &'static str,
        current_price: &'static str,
        long: bool,
        leverage: Uint128,
        expected_result: Result<SignedDecimal, ContractError>,
    }

    #[test]
    fn test_get_pnl_percent_cases() {
        let test_cases = vec![
            PnlPercentTestCase {
                description: "PnL for long position with positive profit",
                open_price: "100",
                current_price: "120",
                long: true,
                leverage: Uint128::new(10),
                expected_result: Ok(SignedDecimal::percent(200)), // 20% * 10x leverage = 200%
            },
            PnlPercentTestCase {
                description: "PnL for long position with loss",
                open_price: "100",
                current_price: "80",
                long: true,
                leverage: Uint128::new(10),
                expected_result: Ok(SignedDecimal::percent(-100)), // -20% * 10x leverage = -200% -> -100%
            },
            PnlPercentTestCase {
                description: "PnL for long position hitting treshold for long",
                open_price: "100",
                current_price: "195",
                long: true,
                leverage: Uint128::new(10),
                expected_result: Ok(SignedDecimal::percent(900)),
            },
            PnlPercentTestCase {
                description: "PnL for short position with positive profit",
                open_price: "100",
                current_price: "20",
                long: false,
                leverage: Uint128::new(10),
                expected_result: Ok(SignedDecimal::percent(800)), // 80% * 10x leverage = 800%
            },
            PnlPercentTestCase {
                description: "PnL for short position with loss",
                open_price: "100",
                current_price: "120",
                long: false,
                leverage: Uint128::new(10),
                expected_result: Ok(SignedDecimal::percent(-100)), // -20% * 10x leverage = -200% -> -100%
            },
        ];

        for test in test_cases {
            let result = get_pnl_percent(
                test.open_price.parse::<Decimal>().unwrap(),
                test.current_price.parse::<Decimal>().unwrap(),
                test.long,
                test.leverage,
            );
            assert_eq!(
                result, test.expected_result,
                "Failed test: {}",
                test.description
            );
        }
    }
}
