use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Env, MessageInfo, Response};
use cw_storage_plus::Item;
use std::collections::HashMap;

use crate::{error::ContractError, trade};

#[cw_serde]
pub struct Trader {
    pub leverage_unlocked: u64,
    pub referral: Addr,
    pub referral_rewards_total: u128,
}

#[cw_serde]
pub struct Trade {
    pub trader: Addr,
    pub pair_index: u64,
    pub initial_pos_token: Decimal256,
    pub position_size_nusd: Decimal256,
    pub open_price: Decimal256,
    pub buy: bool,
    pub leverage: u64,
    pub tp: Decimal256,
    pub sl: Decimal256,
}

#[cw_serde]
pub struct TradeInfo {
    pub token_id: u64,
    pub token_price_nusd: u128,
    pub open_interest_nusd: u128,
    pub tp_last_updated: u64,
    pub sl_last_updated: u64,
    pub being_market_closed: bool,
}

#[cw_serde]
pub struct OpenLimitOrder {
    pub trader: Addr,
    pub pair_index: u64,
    pub index: u64,
    pub position_size: Decimal256,
    pub spread_reduction_p: u64,
    pub buy: bool,
    pub leverage: u64,
    pub tp: Decimal256,
    pub sl: Decimal256,
    pub min_price: Decimal256,
    pub max_price: Decimal256,
    pub block: u64,
    pub token_id: u64,
}

#[cw_serde]
pub struct PendingMarketOrder {
    pub trade: Trade,
    pub block: u64,
    pub wanted_price: Decimal256,
    pub slippage_p: u64,
    pub spread_reduction_p: Decimal256,
    pub token_id: u64,
}

#[cw_serde]
pub struct PendingNftOrder {
    pub nft_holder: Addr,
    pub nft_id: u64,
    pub trader: Addr,
    pub pair_index: u64,
    pub index: u64,
    pub order_type: LimitOrder,
}

#[cw_serde]
pub struct PairParams {
    pub one_percent_depth_above: u128,
    pub one_percent_depth_below: u128,
    pub rollover_fee_per_block_p: u128,
    pub funding_fee_per_block_p: u128,
}

#[cw_serde]
pub struct PairFundingFees {
    pub acc_per_oi_long: i128,
    pub acc_per_oi_short: i128,
    pub last_update_block: u64,
}

#[cw_serde]
pub struct PairRolloverFees {
    pub acc_per_collateral: u128,
    pub last_update_block: u64,
}

#[cw_serde]
pub struct TradeInitialAccFees {
    pub rollover: u128,
    pub funding: i128,
    pub opened_after_update: bool,
}

#[cw_serde]
pub struct AggregatorAnswer {
    pub order_id: u64,
    pub price: u128,
    pub spread_p: u64,
}

#[cw_serde]
pub struct Values {
    pub price: u128,
    pub profit_p: i128,
    pub token_price_nusd: u128,
    pub pos_token: u128,
    pub pos_nusd: u128,
    pub nft_reward: u128,
}

#[cw_serde]
pub enum LimitOrder {
    TP,
    SL,
    LIQ,
    OPEN,
}

#[cw_serde]
pub enum OpenLimitOrderType {
    /// Market ordcers, order is opened as long as the price is within the
    /// limits of the order.
    MARKET,

    /// Reversal limit order, order is opened when the price goes beyond the
    /// limits of the order in the opposite direction.
    REVERSAL,

    /// Momentum limit order, order is opened when the price goes beyond the
    /// limits of the order in the same direction.
    MOMENTUM,
}

pub const STATE: Item<State> = Item::new("state");

///TODO: make this cleaner with sub-structs
#[cw_serde]
pub struct State {
    // user info mapping
    pub traders: HashMap<Addr, Trader>,
    pub oracle_address: Addr,

    // trade mappings. trader, pair_index, trade_index -> trade/trade_info
    // We do hashmaps of hashmaps to be able to count the number of trades
    // for a given trader/pair_index efficiently
    pub open_trades: HashMap<(Addr, u64), HashMap<u64, Trade>>,
    pub open_trades_info: HashMap<(Addr, u64), HashMap<u64, TradeInfo>>,

    // limit orders mappings
    pub open_limit_orders: HashMap<(Addr, u64), HashMap<u64, OpenLimitOrder>>,

    // list of open trades & limit orders
    pub pair_traders: HashMap<u64, Vec<Addr>>,
    pub pair_traders_id: HashMap<(Addr, u64), u64>,

    // Current and max open interests for each pair
    pub open_interest: HashMap<u64, [Decimal256; 3]>, // [long, short, max]

    // Restrictions & Timelocks
    pub trades_per_block: HashMap<u64, u64>,

    // pair infos
    pub max_negative_pnl_on_open_p: Decimal256,
    pub trade_initial_acc_fees: HashMap<(Addr, u64, u64), TradeInitialAccFees>,

    // trading variables
    pub max_trades_per_pair: u64,
    pub max_trade_per_block: u64,
    pub max_pending_market_orders: u64,
    pub max_gain_p: Decimal256,
    pub max_sl_p: Decimal256,
    pub default_leverage_unlocked: u64,
    pub spread_reductions_p: [Decimal256; 5],

    // trading callbacks
    pub max_position_size_nusd: Decimal256,
    pub limit_orders_timelock: u64,
    pub market_orders_timeout: u64,

    pub is_paused: bool, // prevent opening new trade
    pub is_done: bool,   // prevent any interaction with the contract
    pub vault_fee_p: Decimal256,

    // dev gov fees
    pub dev_gov_fees: Decimal256,

    // pair storage
    pub min_lev_pos: HashMap<u64, Decimal256>,
    pub min_leverage: HashMap<u64, u64>,
    pub max_leverage: HashMap<u64, u64>,
    pub pair_params: HashMap<u64, PairParams>,
    pub pair_funding_fees: HashMap<u64, PairFundingFees>,
    pub pair_rollover_fees: HashMap<u64, PairRolloverFees>,
}

impl State {
    pub fn new() -> Self {
        Self {
            traders: HashMap::new(),
            open_trades: HashMap::new(),
            open_trades_info: HashMap::new(),
            open_limit_orders: HashMap::new(),
            pair_traders: HashMap::new(),
            pair_traders_id: HashMap::new(),
            open_interest: HashMap::new(),
            trades_per_block: HashMap::new(),
            max_negative_pnl_on_open_p: Decimal256::from_ratio(40_u64, 100_u64),
            pair_params: HashMap::new(),
            pair_funding_fees: HashMap::new(),
            pair_rollover_fees: HashMap::new(),
            trade_initial_acc_fees: HashMap::new(),
            max_trades_per_pair: 3,
            max_trade_per_block: 5,
            max_pending_market_orders: 5,
            max_gain_p: Decimal256::from_ratio(900_u64, 100_u64),
            max_sl_p: Decimal256::from_ratio(80_u64, 100_u64),
            default_leverage_unlocked: 50_u64,
            spread_reductions_p: [
                Decimal256::from_ratio(15_u64, 100_u64),
                Decimal256::from_ratio(20_u64, 100_u64),
                Decimal256::from_ratio(25_u64, 100_u64),
                Decimal256::from_ratio(30_u64, 100_u64),
                Decimal256::from_ratio(35_u64, 100_u64),
            ],
            max_position_size_nusd: Decimal256::from_atomics(75_000_u128, 0)
                .unwrap(),
            limit_orders_timelock: 30_u64, // 30 blocks
            market_orders_timeout: 30_u64, // 30 blocks

            is_paused: false,
            is_done: false,
            vault_fee_p: Decimal256::from_ratio(10_u64, 100_u64),

            min_lev_pos: HashMap::new(),
            min_leverage: HashMap::new(),
            max_leverage: HashMap::new(),

            oracle_address: Addr::unchecked(""),
        }
    }

    pub fn first_empty_open_limit_index(
        &self,
        address: Addr,
        pair_index: u64,
    ) -> u64 {
        match self.open_limit_orders.get(&(address, pair_index)) {
            None => 0,
            Some(orders) => {
                for i in 0..self.max_trades_per_pair {
                    if !orders.contains_key(&i) {
                        return i;
                    }
                }
                self.max_trades_per_pair
            }
        }
    }

    pub fn store_open_limit_order(
        &mut self,
        info: &MessageInfo,
        trade: &Trade,
        spread_reduction_id: u64,
        env: &Env,
    ) {
        let info = info.clone();
        let index = self
            .first_empty_open_limit_index(info.clone().sender, trade.pair_index);

        let open_limit_order = OpenLimitOrder {
            trader: info.sender.clone(),
            pair_index: trade.pair_index,
            index,
            position_size: trade.position_size_nusd,
            spread_reduction_p: spread_reduction_id,
            buy: trade.buy,
            leverage: trade.leverage,
            tp: trade.tp,
            sl: trade.sl,
            min_price: trade.open_price,
            max_price: trade.open_price,
            block: env.block.height,
            token_id: 0,
        };

        self.open_limit_orders
            .entry((info.sender, trade.pair_index))
            .or_insert_with(HashMap::new)
            .insert(index, open_limit_order);
    }

    pub fn execute_market_order(
        &mut self,
        trade: Trade,
        info: MessageInfo,
        env: Env,
        slippage_p: Decimal256,
        spread_reduction: Decimal256,
        price_impact: Decimal256,
    ) -> Result<Response, ContractError> {
        let price: Decimal256 = todo!();
        let price_after_impact: Decimal256 = todo!();
        let wanted_price: Decimal256 = todo!();

        let mut trade = trade.clone();
        trade.open_price = price_after_impact;

        let max_slippage = wanted_price * slippage_p;

        if price.is_zero()
            || (trade.buy && trade.open_price > wanted_price + max_slippage)
            || (!trade.buy && trade.open_price < wanted_price - max_slippage)
            || (!trade.tp.is_zero()
                && ((trade.buy && trade.open_price >= trade.tp)
                    || (!trade.buy && trade.open_price <= trade.tp)))
            || (!trade.sl.is_zero()
                && ((trade.buy && trade.open_price <= trade.sl)
                    || (!trade.buy && trade.open_price >= trade.sl)))
            || !within_exposure_limits(
                trade.pair_index,
                trade.buy,
                trade.position_size_nusd,
                trade.leverage,
            )
            || price_impact
                .checked_mul(
                    Decimal256::from_atomics(trade.leverage, 0).unwrap(),
                )
                .unwrap()
                > self.max_negative_pnl_on_open_p
        {
            return Err(ContractError::FailedToOpenTrade);
        }

        return self.register_trade(trade, order_type, limit_index);
    }

    pub fn register_trade(
        &mut self,
        trade: Trade,
        nft_id: u64,
        limit_index: u64,
        env: &Env,
        info: &MessageInfo,
    ) -> Result<Response, ContractError> {
        let mut trade = trade.clone();

        // Handle developer and governance fees
        let leverage = Decimal256::from_atomics(trade.leverage, 0).unwrap();
        let fee = trade
            .position_size_nusd
            .checked_mul(leverage)
            .unwrap()
            .checked_mul(self.dev_gov_fees)
            .unwrap();
        trade.position_size_nusd =
            trade.position_size_nusd.checked_sub(fee).unwrap();

        // Receive DAI from trader
        self.receive_dai_from_trader(&trade.trader, trade.position_size_nusd)?;

        // Calculate token price and initial position tokens
        let token_price_nusd = self.get_token_price_nusd();
        trade.initial_pos_token = trade
            .position_size_nusd
            .checked_mul(Decimal256::from_ratio(1_u64, token_price_nusd))
            .unwrap();
        trade.position_size_nusd = Decimal256::zero();

        // Handle referral rewards
        if let Some(referral) = self.get_referral(&trade.trader) {
            let r_tokens = trade
                .initial_pos_token
                .checked_mul(leverage)
                .unwrap()
                .checked_mul(self.get_pair_referral_fee(trade.pair_index))
                .unwrap()
                .checked_div(Decimal256::from_ratio(100_u64, 1_u64))
                .unwrap();
            self.handle_tokens(&referral, r_tokens, true)?;
            self.increase_referral_rewards(&referral, r_tokens);
            trade.initial_pos_token =
                trade.initial_pos_token.checked_sub(r_tokens).unwrap();
        }

        // Handle NFT rewards
        if nft_id < 1500 {
            let n_tokens = trade
                .initial_pos_token
                .checked_mul(leverage)
                .unwrap()
                .checked_mul(self.get_pair_nft_limit_order_fee(trade.pair_index))
                .unwrap()
                .checked_div(Decimal256::from_ratio(100_u64, 1_u64))
                .unwrap();
            trade.initial_pos_token =
                trade.initial_pos_token.checked_sub(n_tokens).unwrap();
            self.distribute_nft_reward(
                trade.trader.clone(),
                trade.pair_index,
                limit_index,
                n_tokens,
            )?;
            self.increase_nft_rewards(nft_id, n_tokens);
        }

        // Assign trade index and update TP/SL
        trade.index =
            self.first_empty_trade_index(trade.trader.clone(), trade.pair_index);
        trade.tp =
            self.correct_tp(trade.open_price, leverage, trade.tp, trade.buy);
        trade.sl =
            self.correct_sl(trade.open_price, leverage, trade.sl, trade.buy);

        // Store initial accumulated fees and update group collateral
        self.store_trade_initial_acc_fees(
            &trade.trader,
            trade.pair_index,
            trade.index,
            trade.buy,
        );
        self.update_group_collateral(
            trade.pair_index,
            trade
                .initial_pos_token
                .checked_mul(Decimal256::from_ratio(token_price_nusd, 1_u64))
                .unwrap(),
            trade.buy,
            true,
        );

        // Store the trade
        self.store_trade(
            trade.clone(),
            TradeInfo {
                token_id: 0,
                token_price_nusd,
                open_interest_nusd: trade
                    .initial_pos_token
                    .checked_mul(leverage)
                    .unwrap()
                    .checked_mul(Decimal256::from_ratio(token_price_nusd, 1_u64))
                    .unwrap(),
                tp_last_updated: env.block.height,
                sl_last_updated: env.block.height,
                being_market_closed: false,
            },
        );

        Ok(Response::new().add_attribute("action", "register_trade"))
    }

    // Mock implementations for external methods. These should be implemented based on actual logic.
    fn receive_dai_from_trader(
        &self,
        _trader: &Addr,
        _amount: Decimal256,
    ) -> Result<(), ContractError> {
        // Logic to receive DAI from trader
        Ok(())
    }

    fn get_token_price_nusd(&self) -> u64 {
        // Logic to get the token price in NUSD
        1 // Placeholder
    }

    fn get_referral(&self, _trader: &Addr) -> Option<Addr> {
        // Logic to get referral address
        None // Placeholder
    }

    fn handle_tokens(
        &self,
        _address: &Addr,
        _amount: Decimal256,
        _mint: bool,
    ) -> Result<(), ContractError> {
        // Logic to handle tokens
        Ok(())
    }

    fn increase_referral_rewards(&self, _address: &Addr, _amount: Decimal256) {
        // Logic to increase referral rewards
    }

    fn distribute_nft_reward(
        &self,
        _trader: Addr,
        _pair_index: u64,
        _limit_index: u64,
        _n_tokens: Decimal256,
    ) -> Result<(), ContractError> {
        // Logic to distribute NFT rewards
        Ok(())
    }

    fn increase_nft_rewards(&self, _nft_id: u64, _amount: Decimal256) {
        // Logic to increase NFT rewards
    }

    fn first_empty_trade_index(&self, _trader: Addr, _pair_index: u64) -> u64 {
        // Logic to get the first empty trade index
        0 // Placeholder
    }

    fn correct_tp(
        &self,
        _open_price: Decimal256,
        _leverage: Decimal256,
        _tp: Decimal256,
        _buy: bool,
    ) -> Decimal256 {
        // Logic to correct TP
        Decimal256::zero() // Placeholder
    }

    fn correct_sl(
        &self,
        _open_price: Decimal256,
        _leverage: Decimal256,
        _sl: Decimal256,
        _buy: bool,
    ) -> Decimal256 {
        // Logic to correct SL
        Decimal256::zero() // Placeholder
    }

    fn store_trade_initial_acc_fees(
        &self,
        _trader: &Addr,
        _pair_index: u64,
        _index: u64,
        _buy: bool,
    ) {
        // Logic to store initial accumulated fees
    }

    fn update_group_collateral(
        &self,
        _pair_index: u64,
        _amount: Decimal256,
        _buy: bool,
        _increase: bool,
    ) {
        // Logic to update group collateral
    }

    fn store_trade(&self, _trade: Trade, _trade_info: TradeInfo) {
        // Logic to store trade
    }

    fn get_pair_referral_fee(&self, _pair_index: u64) -> Decimal256 {
        // Logic to get pair referral fee
        Decimal256::zero() // Placeholder
    }

    fn get_pair_nft_limit_order_fee(&self, _pair_index: u64) -> Decimal256 {
        // Logic to get pair NFT limit order fee
        Decimal256::zero() // Placeholder
    }
}

fn within_exposure_limits(
    pair_index: u64,
    buy: bool,
    position_size_nusd: Decimal256,
    leverage: u64,
) -> bool {
    todo!()
}
