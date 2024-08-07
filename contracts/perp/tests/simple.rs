use cosmwasm_std::{Addr, Decimal};

use crate::app::App;

mod app;

#[test]
fn long_btc_and_close() {
    // also tests with alt receiver
    let mut app = App::default();
    let _alice = Addr::unchecked("alice");
    let pair_index = 0;
    let _collateral_index = 0;

    app.set_up_oracle_asset(pair_index, u128_to_dec(69_000_u64, 0).unwrap());
    app.set_up_oracle_collateral(pair_index, Decimal::percent(101));
    app.create_default_pairs();

    // we first set up all the state for the trade

    // open a position
    // let alice = Addr::unchecked("alice");

    // let trade = perp::msgs::ExecuteMsg::OpenTrade {
    //     trade: Trade {
    //         user: alice.clone(),
    //         pair_index: 0,
    //         leverage: Uint128::new(10_u128),
    //         long: true,
    //         is_open: true,
    //         collateral_index: 0,
    //         trade_type: TradeType::Trade,
    //         collateral_amount: Uint128::new(1000),
    //         open_price: Decimal::zero(),
    //         tp: Decimal::zero(),
    //         sl: Decimal::zero(),
    //     },
    //     order_type: OpenOrderType::MARKET,
    //     spread_reduction_id: 0,
    //     slippage_p: Decimal::from_ratio(0_u64, 1_u64),
    //     referral: "".to_string(),
    // };

    // cw_multi_test::Executor::execute_contract(
    //     &mut app.simapp,
    //     app.perp_owner.clone(),
    //     app.perp_addr.clone(),
    //     &trade,
    //     &[],
    // )
    // .unwrap();
}
