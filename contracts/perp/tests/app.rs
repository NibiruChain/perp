use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{from_json, Addr, Coin, Decimal, Empty, StdError};
use cw_multi_test::{
    error::AnyResult, BankSudo, Contract, ContractWrapper, Executor,
};
use perp::{msgs::AdminExecuteMsg, trading::state::TradingActivated};
use test_app::Simapp;

struct Contracts {
    oracle: Box<dyn Contract<Empty>>,
    perp: Box<dyn Contract<Empty>>,
    referrals: Box<dyn Contract<Empty>>,
}

impl Contracts {
    fn new() -> Self {
        Contracts {
            oracle: Box::new(ContractWrapper::new(
                oracle::contract::execute,
                oracle::contract::instantiate,
                oracle::contract::query,
            )),
            perp: Box::new(ContractWrapper::new(
                perp::contract::execute,
                perp::contract::instantiate,
                perp::query::query,
            )),
            referrals: Box::new(ContractWrapper::new(
                referrals::contract::execute,
                referrals::contract::instantiate,
                referrals::query::query,
            )),
        }
    }
}

pub struct App {
    pub simapp: Simapp,
    pub oracle_addr: Addr,
    pub perp_addr: Addr,
    pub referrals_addr: Addr,

    pub oracle_owner: Addr,
    pub referrals_owner: Addr,
    pub perp_owner: Addr,
}

impl Default for App {
    fn default() -> Self {
        let mut app = test_app::new();
        let contracts = Contracts::new();
        let oracle_code_id = app.store_code(contracts.oracle);
        let perp_code_id = app.store_code(contracts.perp);
        let referrals_code_id = app.store_code(contracts.referrals);

        let oracle_owner = Addr::unchecked("oracle");
        let oracle = app
            .instantiate_contract(
                oracle_code_id,
                oracle_owner.clone(),
                &oracle::contract::OracleInstantiateMsg {
                    owner: Some(oracle_owner.clone().into_string()),
                },
                &[],
                "oracle",
                None,
            )
            .unwrap();

        let referrals_owner = Addr::unchecked("oracle");
        let referrals = app
            .instantiate_contract(
                referrals_code_id,
                referrals_owner.clone(),
                &referrals::contract::ReferralInstantiateMsg {},
                &[],
                "referrals",
                None,
            )
            .unwrap();

        let perp_owner = Addr::unchecked("perp");
        let perp = app
            .instantiate_contract(
                perp_code_id,
                perp_owner.clone(),
                &perp::msgs::InstantiateMsg {
                    owner: Some(perp_owner.clone().into_string()),
                    oracle_address: Some(oracle.to_string()),
                    staking_address: None,
                },
                &[],
                "perp",
                None,
            )
            .unwrap();

        App {
            simapp: app,
            oracle_addr: oracle,
            perp_addr: perp,
            referrals_addr: referrals,
            oracle_owner,
            referrals_owner,
            perp_owner,
        }
    }
}

impl App {
    pub fn execute<T: DeserializeOwned>(
        &mut self,
        from: &Addr,
        msg: perp::msgs::ExecuteMsg,
        funds: Vec<Coin>,
    ) -> AnyResult<T> {
        let res = self.simapp.execute_contract(
            from.clone(),
            self.perp_addr.clone(),
            &msg,
            &funds,
        )?;
        let data = res.data.ok_or(StdError::generic_err("expected data"))?;
        Ok(from_json::<T>(data)?)
    }

    pub fn fund(&mut self, addr: &Addr, coins: &Vec<Coin>) {
        self.simapp
            .sudo(
                BankSudo::Mint {
                    to_address: addr.into(),
                    amount: coins.clone(),
                }
                .into(),
            )
            .unwrap();
    }

    pub fn set_up_oracle_asset(&mut self, index: u64, price: Decimal) {
        let oracle_post_price =
            oracle::contract::OraclesExecuteMsg::SetPrice { index, price };

        self.simapp
            .execute_contract(
                self.oracle_owner.clone(),
                self.oracle_addr.clone(),
                &oracle_post_price,
                &[],
            )
            .unwrap();
    }

    pub fn set_up_oracle_collateral(&mut self, index: u64, price: Decimal) {
        let oracle_post_price =
            oracle::contract::OraclesExecuteMsg::SetCollateralPrice {
                index,
                price,
            };

        self.simapp
            .execute_contract(
                self.oracle_owner.clone(),
                self.oracle_addr.clone(),
                &oracle_post_price,
                &[],
            )
            .unwrap();
    }

    pub fn create_default_pairs(&mut self) {
        let mut messages: Vec<AdminExecuteMsg> = vec![];

        // pairs
        messages.push(AdminExecuteMsg::default_set_pairs());
        messages.push(AdminExecuteMsg::default_set_groups());
        // fees
        messages.push(AdminExecuteMsg::default_set_fees());
        messages.push(AdminExecuteMsg::default_set_fee_tiers());
        // trading
        messages.push(AdminExecuteMsg::default_collaterals());

        // turn on trading
        messages.push(AdminExecuteMsg::set_trading_activated(
            TradingActivated::Activated,
        ));

        for msg in messages {
            self.simapp
                .execute_contract(
                    self.perp_owner.clone(),
                    self.perp_addr.clone(),
                    &perp::msgs::ExecuteMsg::AdminMsg { msg },
                    &[],
                )
                .unwrap();
        }
    }
}
