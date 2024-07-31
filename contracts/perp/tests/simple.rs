use cosmwasm_std::Addr;

use crate::app::App;

mod app;

#[test]
fn long_btc_and_close() {
    // also tests with alt receiver
    let mut app = App::default();
    let alice = Addr::unchecked("alice");

    app.create_default_pairs();

    // we first set up all the state for the trade
}
