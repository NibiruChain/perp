use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, BlockInfo, Decimal, Deps, SignedDecimal, Timestamp,
    Uint128,
};
use cw_storage_plus::{Item, Map};

use crate::{
    borrowing::{get_trade_borrowing_fees, state::BorrowingFeeInput},
    constants::LIQ_THRESHOLD_P,
    error::ContractError,
    utils::{u128_to_dec, u128_to_i128, u128_to_sdec},
};

pub const COLLATERALS: Map<u64, String> = Map::new("collaterals");
pub const TRADES: Map<(Addr, u64), Trade> = Map::new("trades");
pub const TRADE_INFOS: Map<(Addr, u64), TradeInfo> = Map::new("trade_infos");
pub const TRADER_STORED: Map<Addr, bool> = Map::new("trader_stored");
pub const USER_COUNTERS: Map<Addr, u64> = Map::new("user_counters");

// todo: make message for this
pub const TRADING_ACTIVATED: Item<TradingActivated> =
    Item::new("trading_activated");

#[cw_serde]
pub enum TradingActivated {
    Activated,
    CloseOnly,
    Paused,
}

#[cw_serde]
pub struct Trader {
    pub leverage_unlocked: u64,
    pub referral: Addr,
    pub referral_rewards_total: u128,
}

#[cw_serde]
pub struct Trade {
    pub user: Addr,
    pub pair_index: u64,
    pub index: u64,

    pub leverage: Uint128,
    pub long: bool,
    pub is_open: bool,
    pub collateral_index: u64,
    pub trade_type: TradeType,
    pub collateral_amount: Uint128,

    pub open_price: Decimal,

    pub tp: Decimal,
    pub sl: Decimal,
}

impl Trade {
    pub fn get_position_size_collateral(&self) -> Uint128 {
        self.collateral_amount.checked_mul(self.leverage).unwrap()
    }

    pub fn get_trade_value_collateral(
        &self,
        deps: &Deps,
        block: &BlockInfo,
        percent_profit: SignedDecimal,
        closing_fee_collateral: Uint128,
        order_type: PendingOrderType,
    ) -> Result<(Uint128, Uint128), ContractError> {
        let borrowing_fees_collateral =
            self.get_trade_borrowing_fees_collateral(deps, block)?;

        let value_collateral = if order_type == PendingOrderType::LiqClose {
            Uint128::zero()
        } else {
            let value = u128_to_i128(self.collateral_amount)?
                + (u128_to_sdec(self.collateral_amount)?
                    .checked_mul(percent_profit)?
                    .to_int_floor())
                .checked_sub(u128_to_i128(borrowing_fees_collateral)?)?
                .checked_sub(u128_to_i128(closing_fee_collateral)?)?;

            let collateral_liq_threshold = u128_to_dec(self.collateral_amount)?
                .checked_mul(Decimal::one().checked_sub(LIQ_THRESHOLD_P)?)?
                .to_uint_floor();

            if value.i128() > collateral_liq_threshold.u128() as i128 {
                Uint128::try_from(value).unwrap()
            } else {
                Uint128::zero()
            }
        };

        Ok((value_collateral, borrowing_fees_collateral))
    }

    fn get_trade_borrowing_fees_collateral(
        &self,
        deps: &Deps,
        block: &BlockInfo,
    ) -> Result<Uint128, ContractError> {
        let input = BorrowingFeeInput {
            collateral_index: self.collateral_index,
            trader: self.user.clone(),
            pair_index: self.pair_index,
            index: self.index,
            long: self.long,
            collateral: self.collateral_amount,
            leverage: self.leverage,
        };

        get_trade_borrowing_fees(deps, block, input)
    }
}

#[cw_serde]
pub struct TradeInfo {
    pub created_block: u64,
    pub tp_last_updated_block: u64,
    pub sl_last_updated_block: u64,
    pub max_slippage_p: Decimal,
    pub last_oi_update_ts: Timestamp,
    pub collateral_price_usd: Decimal, // collateral price at trade open
}

#[cw_serde]
pub enum TradeType {
    Trade,
    Limit,
    Stop,
}

#[cw_serde]
pub enum PendingOrderType {
    Market,
    LimitOpen,
    StopOpen,
    TpClose,
    SlClose,
    LiqClose,
}

#[cw_serde]
pub struct Values {
    pub price: u128,
    pub profit_p: i128,
    pub token_price_nusd: u128,
    pub pos_token: u128,
    pub pos_nusd: u128,
}

#[cw_serde]
pub enum LimitOrder {
    TP,
    SL,
    LIQ,
    OPEN,
}

#[cw_serde]
pub enum OpenOrderType {
    /// Market orders, order is opened as long as the price is within the
    /// limits of the order.
    MARKET,

    /// Reversal limit order, order is opened when the price goes beyond the
    /// limits of the order in the opposite direction.
    REVERSAL,

    /// Momentum limit order, order is opened when the price goes beyond the
    /// limits of the order in the same direction.
    MOMENTUM,
}
