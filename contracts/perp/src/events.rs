use cosmwasm_std::Event;

pub fn event_toggle_halt(is_halted: &bool) -> Event {
    Event::new("broker_bank/toggle_halt")
        .add_attribute("new_is_halted", is_halted.to_string())
}

pub fn event_manager_updated(manager: &str) -> Event {
    Event::new("gns_pair_infos_v6_1/manager_updated")
        .add_attribute("manager", manager)
}

pub fn event_max_negative_pnl_on_open_p_updated(value: &u128) -> Event {
    Event::new("gns_pair_infos_v6_1/max_negative_pnl_on_open_p_updated")
        .add_attribute("value", value.to_string())
}

pub fn event_pair_params_updated(pair_index: &u64, value: &str) -> Event {
    Event::new("gns_pair_infos_v6_1/pair_params_updated")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("value", value)
}

pub fn event_one_percent_depth_updated(
    pair_index: &u64,
    value_above: &u128,
    value_below: &u128,
) -> Event {
    Event::new("gns_pair_infos_v6_1/one_percent_depth_updated")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("value_above", value_above.to_string())
        .add_attribute("value_below", value_below.to_string())
}

pub fn event_rollover_fee_per_block_p_updated(
    pair_index: &u64,
    value: &u128,
) -> Event {
    Event::new("gns_pair_infos_v6_1/rollover_fee_per_block_p_updated")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("value", value.to_string())
}

pub fn event_funding_fee_per_block_p_updated(
    pair_index: &u64,
    value: &u128,
) -> Event {
    Event::new("gns_pair_infos_v6_1/funding_fee_per_block_p_updated")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("value", value.to_string())
}

pub fn event_trade_initial_acc_fees_stored(
    trader: &str,
    pair_index: &u64,
    index: &u64,
    rollover: &u128,
    funding: &i128,
) -> Event {
    Event::new("gns_pair_infos_v6_1/trade_initial_acc_fees_stored")
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("index", index.to_string())
        .add_attribute("rollover", rollover.to_string())
        .add_attribute("funding", funding.to_string())
}

pub fn event_acc_funding_fees_stored(
    pair_index: &u64,
    value_long: &i128,
    value_short: &i128,
) -> Event {
    Event::new("gns_pair_infos_v6_1/acc_funding_fees_stored")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("value_long", value_long.to_string())
        .add_attribute("value_short", value_short.to_string())
}

pub fn event_acc_rollover_fees_stored(pair_index: &u64, value: &u128) -> Event {
    Event::new("gns_pair_infos_v6_1/acc_rollover_fees_stored")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("value", value.to_string())
}

pub fn event_fees_charged(
    pair_index: &u64,
    long: &bool,
    collateral: &u128,
    leverage: &u64,
    percent_profit: &i128,
    rollover_fees: &u128,
    funding_fees: &i128,
) -> Event {
    Event::new("gns_pair_infos_v6_1/fees_charged")
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("long", long.to_string())
        .add_attribute("collateral", collateral.to_string())
        .add_attribute("leverage", leverage.to_string())
        .add_attribute("percent_profit", percent_profit.to_string())
        .add_attribute("rollover_fees", rollover_fees.to_string())
        .add_attribute("funding_fees", funding_fees.to_string())
}

pub fn event_trade_market_executed(
    order_id: &u64,
    trader: &str,
    pair_index: &u64,
    open: &bool,
    price: &u128,
    price_impact_p: &u128,
    position_size_dai: &u128,
    percent_profit: &i128,
    dai_sent_to_trader: &u128,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/market_executed")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("open", open.to_string())
        .add_attribute("price", price.to_string())
        .add_attribute("price_impact_p", price_impact_p.to_string())
        .add_attribute("position_size_dai", position_size_dai.to_string())
        .add_attribute("percent_profit", percent_profit.to_string())
        .add_attribute("dai_sent_to_trader", dai_sent_to_trader.to_string())
}

pub fn event_limit_executed(
    order_id: &u64,
    limit_index: &u64,
    trader: &str,
    pair_index: &u64,
    nft_holder: &str,
    order_type: &str,
    price: &u128,
    price_impact_p: &u128,
    position_size_dai: &u128,
    percent_profit: &i128,
    dai_sent_to_trader: &u128,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/limit_executed")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("limit_index", limit_index.to_string())
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("nft_holder", nft_holder)
        .add_attribute("order_type", order_type)
        .add_attribute("price", price.to_string())
        .add_attribute("price_impact_p", price_impact_p.to_string())
        .add_attribute("position_size_dai", position_size_dai.to_string())
        .add_attribute("percent_profit", percent_profit.to_string())
        .add_attribute("dai_sent_to_trader", dai_sent_to_trader.to_string())
}

pub fn event_market_open_canceled(
    order_id: &u64,
    trader: &str,
    pair_index: &u64,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/market_open_canceled")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
}

pub fn event_market_close_canceled(
    order_id: &u64,
    trader: &str,
    pair_index: &u64,
    index: &u64,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/market_close_canceled")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("index", index.to_string())
}

pub fn event_sl_updated(
    order_id: &u64,
    trader: &str,
    pair_index: &u64,
    index: &u64,
    new_sl: &u128,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/sl_updated")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("index", index.to_string())
        .add_attribute("new_sl", new_sl.to_string())
}

pub fn event_sl_canceled(
    order_id: &u64,
    trader: &str,
    pair_index: &u64,
    index: &u64,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/sl_canceled")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("index", index.to_string())
}

pub fn event_liquidate(
    trader: &str,
    pair_index: &u64,
    index: &u64,
    price: &u128,
    liq_id: &u64,
) -> Event {
    Event::new("gns_trading_callbacks_v6_1/liquidate")
        .add_attribute("trader", trader)
        .add_attribute("pair_index", pair_index.to_string())
        .add_attribute("index", index.to_string())
        .add_attribute("price", price.to_string())
        .add_attribute("liq_id", liq_id.to_string())
}
