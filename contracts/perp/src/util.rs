use std::collections::HashMap;

use cosmwasm_std::{Decimal256, MessageInfo, Response};

use crate::{
    error::ContractError,
    state::{State, Trade},
};

pub fn get_trade_price_impact(
    _pair_index: u64,
    _buy: bool,
    _leverage: Decimal256,
) -> Decimal256 {
    todo!()
}

pub fn validate_order(
    state: &State,
    info: &MessageInfo,
    trade: &Trade,
) -> Result<Option<Response>, ContractError> {
    let info = info.clone();
    let key = (info.sender, trade.pair_index);

    // check count of orders
    if state.open_trades.get(&key).unwrap_or(&HashMap::new()).len()
        + state
            .open_limit_orders
            .get(&key)
            .unwrap_or(&HashMap::new())
            .len()
        >= state.max_trades_per_pair as usize
    {
        return Err(ContractError::MaxTradesPerPair);
    }

    if trade.position_size_nusd > state.max_position_size_nusd {
        return Err(ContractError::InvalidPositionSize);
    }
    if trade
        .position_size_nusd
        .checked_mul(Decimal256::from_atomics(trade.leverage, 0).unwrap())
        .unwrap()
        < *state
            .min_lev_pos
            .get(&trade.pair_index)
            .unwrap_or(&Decimal256::zero())
    {
        return Err(ContractError::InvalidLeverage);
    }
    if trade.leverage < 0
        || trade.leverage
            >= *state.min_leverage.get(&trade.pair_index).unwrap_or(&0)
        || trade.leverage
            <= *state.max_leverage.get(&trade.pair_index).unwrap_or(&0)
    {
        return Err(ContractError::InvalidLeverage);
    }
    if !trade.tp.is_zero()
        && ((trade.buy && trade.tp <= trade.open_price)
            || (!trade.buy && trade.tp >= trade.open_price))
    {
        return Err(ContractError::InvalidTpSl);
    }

    if !trade.sl.is_zero()
        && ((trade.buy && trade.sl >= trade.open_price)
            || (!trade.buy && trade.sl <= trade.open_price))
    {
        return Err(ContractError::InvalidTpSl);
    }
    Ok(None)
}
