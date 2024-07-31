use std::collections::HashMap;

use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{from_json, Addr, Coin, Decimal, Empty, StdError};
use cw_multi_test::{
    error::AnyResult, BankSudo, Contract, ContractWrapper, Executor,
};
use perp::pairs::state::PAIRS;
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

    pub fn create_default_pairs(&mut self) {
        let mut pairs: HashMap<u64, perp::pairs::state::Pair> = HashMap::new();

        pairs.insert(
            0,
            perp::pairs::state::Pair {
                from: "btc".to_string(),
                to: "usd".to_string(),
                spread_p: Decimal::zero(),
                oracle_index: 0,
                group_index: 0,
                fee_index: 0,
            },
        );

        pairs.insert(
            1,
            perp::pairs::state::Pair {
                from: "eth".to_string(),
                to: "usd".to_string(),
                spread_p: Decimal::zero(),
                oracle_index: 0,
                group_index: 0,
                fee_index: 0,
            },
        );

        let message = perp::msgs::AdminExecuteMsg::SetPairs {
            pairs: pairs.clone(),
        };
        self.simapp
            .execute_contract(
                self.perp_owner.clone(),
                self.perp_addr.clone(),
                &perp::msgs::ExecuteMsg::AdminMsg { msg: message },
                &vec![],
            )
            .unwrap();

        // assert pairs are created
        let storage = &*self.simapp.contract_storage(&self.perp_addr);
        let pair_1 = PAIRS.load(storage, 0).unwrap();
        let pair_2 = PAIRS.load(storage, 1).unwrap();

        assert_eq!(&pair_1, pairs.get(&0).unwrap());
        assert_eq!(&pair_2, pairs.get(&1).unwrap());

        // asset pair are listed
    }
}
