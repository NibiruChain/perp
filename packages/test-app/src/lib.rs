use cosmwasm_std::{
    from_json,
    testing::{MockApi, MockStorage},
    to_json_binary, Addr, Coin, Empty, WasmMsg,
};
use cw_multi_test::WasmKeeper;
use cw_multi_test::{
    error::AnyResult, no_init, App, BankKeeper, BankSudo, BasicAppBuilder,
    DistributionKeeper, Executor, FailingModule, GovFailingModule,
    IbcFailingModule, StakeKeeper,
};
use cw_utils::parse_instantiate_response_data;
use serde::{de::DeserializeOwned, Serialize};

pub type Simapp<ExecC = Empty, QueryC = Empty> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    StakeKeeper,
    DistributionKeeper,
    IbcFailingModule,
    GovFailingModule,
>;

pub trait SimappExtension {
    fn instantiate_contract_with_data<
        T: Serialize,
        U: Into<String>,
        Z: DeserializeOwned,
    >(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[Coin],
        label: U,
        admin: Option<String>,
    ) -> AnyResult<(Addr, Z)>;
}

impl SimappExtension for Simapp {
    fn instantiate_contract_with_data<
        T: Serialize,
        U: Into<String>,
        Z: DeserializeOwned,
    >(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[Coin],
        label: U,
        admin: Option<String>,
    ) -> AnyResult<(Addr, Z)> {
        // instantiate contract
        let init_msg = to_json_binary(init_msg)?;
        let msg = WasmMsg::Instantiate {
            admin,
            code_id,
            msg: init_msg,
            funds: send_funds.to_vec(),
            label: label.into(),
        };
        let res = self.execute(sender, msg.into())?;
        let data = parse_instantiate_response_data(
            res.data.unwrap_or_default().as_slice(),
        )?;
        let inst_resp = data.data.ok_or(anyhow::anyhow!("empty data"))?;
        Ok((
            Addr::unchecked(data.contract_address),
            from_json(inst_resp)?,
        ))
    }
}

pub fn new() -> Simapp {
    BasicAppBuilder::new_custom().build(no_init)
}

pub fn fund(app: &mut Simapp, addr: Addr, coins: &[Coin]) {
    app.sudo(
        BankSudo::Mint {
            to_address: addr.into(),
            amount: coins.to_vec(),
        }
        .into(),
    )
    .unwrap();
}

#[cfg(test)]
mod test {
    #[test]
    fn builds() {}
}
