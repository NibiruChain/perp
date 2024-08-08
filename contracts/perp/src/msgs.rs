use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::{
    borrowing::state::{BorrowingData, BorrowingPairGroup, OpenInterest},
    fees::state::{FeeTier, TraderDailyInfo},
    pairs::state::{Fee, Group, Pair},
    price_impact::state::{OiWindowsSettings, PairDepth, PairOi},
    trading::state::{
        OpenOrderType, PendingOrderType, Trade, TradeInfo, TradingActivated,
    },
};

#[cw_serde]
pub enum ExecuteMsg {
    /// Opens a new trade with specified parameters.
    /// Parameters:
    /// - trade: Trade details including position size, leverage, and other parameters.
    /// - order_type: The type of order (e.g., legacy, reversal, momentum).
    /// - spread_reduction_id: ID for any spread reduction applicable.
    /// - slippage_p: Slippage percentage for market orders.
    /// - referral: Referral address for tracking referral rewards.
    OpenTrade {
        trade: Trade,
        order_type: OpenOrderType,
        slippage_p: String,
        referral: String,
    },

    /// Closes an open trade for the specified pair index and trade index.
    /// Parameters:
    /// - index: The index of the trade to be closed.
    CloseTradeMarket { index: u64 },

    /// Updates the open limit order with new parameters.
    /// Parameters:
    /// - index: The index of the limit order to update.
    /// - price: New price for the limit order.
    /// - tp: New take profit value.
    /// - sl: New stop loss value.
    /// - slippage_p: New slippage percentage.
    UpdateOpenLimitOrder {
        index: u64,
        price: Decimal,
        tp: Decimal,
        sl: Decimal,
        slippage_p: Decimal,
    },

    /// Cancels the open limit order.
    /// Parameters:
    /// - index: The index of the limit order to cancel.
    CancelOpenLimitOrder { index: u64 },

    /// Updates the take profit value for an open trade.
    /// Parameters:
    /// - index: The index of the trade to update.
    /// - new_tp: The new take profit value.
    UpdateTp { index: u64, new_tp: Decimal },

    /// Updates the stop loss value for an open trade.
    /// Parameters:
    /// - index: The index of the trade to update.
    /// - new_sl: The new stop loss value.
    UpdateSl { index: u64, new_sl: Decimal },

    /// Trigger a limit order.
    /// Parameters:
    /// - trader: Address of the trader.
    /// - index: The index of the trade or order.
    /// - order_type: The type of pending order.
    TriggerTrade {
        trader: Addr,
        index: u64,
        order_type: PendingOrderType,
    },

    /// Admin executes the specified message.
    /// Parameters:
    /// - msg: The admin message to execute.
    AdminMsg { msg: AdminExecuteMsg },
}

#[cw_serde]
pub enum AdminExecuteMsg {
    // Pairs
    SetPairs {
        pairs: HashMap<u64, Pair>,
    },
    SetGroups {
        groups: HashMap<u64, Group>,
    },
    SetFees {
        fees: HashMap<u64, Fee>,
    },
    SetPairCustomMaxLeverage {
        pair_custom_max_leverage: HashMap<u64, Uint128>,
    },
    UpdateOracleAddress {
        oracle_address: String,
    },
    UpdateStakingAddress {
        staking_address: String,
    },

    // Fees
    UpdateFeeTiers {
        fee_tiers: [FeeTier; 8],
    },
    UpdatePendingGovFees {
        pending_gov_fees: HashMap<u64, Uint128>,
    },
    UpdateTraderDailyInfos {
        trader_daily_infos: HashMap<(String, u64), TraderDailyInfo>,
    },

    // Borrowing
    UpdateBorrowingPairs {
        borrowing_pairs: HashMap<(u64, u64), BorrowingData>,
    },
    UpdateBorrowingPairGroups {
        pair_groups: HashMap<(u64, u64), Vec<BorrowingPairGroup>>,
    },
    UpdateBorrowingPairOis {
        pair_ois: HashMap<(u64, u64), OpenInterest>,
    },
    UpdateBorrowingGroups {
        groups: HashMap<(u64, u64), BorrowingData>,
    },
    UpdateBorrowingGroupOis {
        group_ois: HashMap<(u64, u64), OpenInterest>,
    },

    // Price impact
    UpdateOiWindowsSettings {
        oi_windows_settings: OiWindowsSettings,
    },
    UpdateWindows {
        windows: HashMap<(u64, u64, u64), PairOi>,
    },
    UpdatePairDepths {
        pair_depths: HashMap<u64, PairDepth>,
    },

    // Trading
    UpdateCollaterals {
        collaterals: HashMap<u64, String>,
    },
    UpdateTrades {
        trades: HashMap<(Addr, u64), Trade>,
    },
    UpdateTradeInfos {
        trade_infos: HashMap<(Addr, u64), TradeInfo>,
    },
    UpdateTraderStored {
        trader_stored: HashMap<Addr, bool>,
    },
    UpdateTradingActivated {
        trading_activated: TradingActivated,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The owner is the only one that can use ExecuteMsg.
    pub owner: Option<String>,
    pub staking_address: Option<String>,
    pub oracle_address: Option<String>,
}

#[derive(QueryResponses)]
#[cw_serde]
pub enum QueryMsg {
    /// HasOpenLimitOrder returns whether the trader has an open limit order
    #[returns(bool)]
    HasOpenLimitOrder {
        address: String,
        pair_index: u64,
        index: u64,
    },
}

impl AdminExecuteMsg {
    pub fn default_set_pairs() -> Self {
        AdminExecuteMsg::SetPairs {
            pairs: vec![
                (
                    0,
                    Pair {
                        from: "btc".to_string(),
                        to: "usd".to_string(),
                        spread_p: Decimal::zero(),
                        oracle_index: 0,
                        group_index: 0,
                        fee_index: 0,
                    },
                ),
                (
                    1,
                    Pair {
                        from: "eth".to_string(),
                        to: "usd".to_string(),
                        spread_p: Decimal::zero(),
                        oracle_index: 0,
                        group_index: 0,
                        fee_index: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        }
    }
    pub fn default_set_groups() -> Self {
        AdminExecuteMsg::SetGroups {
            groups: vec![(
                0,
                Group {
                    name: "default".to_string(),
                    min_leverage: 1u128.into(),
                    max_leverage: 100u128.into(),
                },
            )]
            .into_iter()
            .collect(),
        }
    }
    pub fn default_set_fees() -> Self {
        AdminExecuteMsg::SetFees {
            fees: vec![(
                0,
                Fee {
                    name: "default".to_string(),
                    open_fee_p: Decimal::zero(),
                    close_fee_p: Decimal::zero(),
                    oracle_fee_p: Decimal::zero(),
                    trigger_order_fee_p: Decimal::zero(),
                    min_position_size_usd: Uint128::zero(),
                },
            )]
            .into_iter()
            .collect(),
        }
    }
    pub fn default_set_fee_tiers() -> Self {
        let fee_tiers: [FeeTier; 8] = [
            FeeTier {
                fee_multiplier: Decimal::from_ratio(975_u64, 1000_u64),
                points_treshold: Uint128::new(6000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(950_u64, 1000_u64),
                points_treshold: Uint128::new(20000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(925_u64, 1000_u64),
                points_treshold: Uint128::new(50000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(900_u64, 1000_u64),
                points_treshold: Uint128::new(100000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(850_u64, 1000_u64),
                points_treshold: Uint128::new(250000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(800_u64, 1000_u64),
                points_treshold: Uint128::new(400000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(700_u64, 1000_u64),
                points_treshold: Uint128::new(1000000000),
            },
            FeeTier {
                fee_multiplier: Decimal::from_ratio(600_u64, 1000_u64),
                points_treshold: Uint128::new(2000000000),
            },
        ];
        AdminExecuteMsg::UpdateFeeTiers {
            fee_tiers: fee_tiers.clone(),
        }
    }
    pub fn default_collaterals() -> Self {
        AdminExecuteMsg::UpdateCollaterals {
            collaterals: vec![(0, "usd".to_string())].into_iter().collect(),
        }
    }
    pub fn set_trading_activated(activated: TradingActivated) -> Self {
        AdminExecuteMsg::UpdateTradingActivated {
            trading_activated: activated,
        }
    }
}
