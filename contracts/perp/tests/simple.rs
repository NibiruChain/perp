use cosmwasm_std::Addr;

use crate::app::App;

mod app;

#[test]
fn success() {
    // also tests with alt receiver
    let mut app = App::default();
    let alice = Addr::unchecked("alice");
}
