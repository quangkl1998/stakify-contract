#[cfg(test)]
pub mod env {
    use cosmwasm_std::{Addr, Empty};
    use cw20::MinterResponse;
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    use crate::contract::{
        execute as FactoryExecute, instantiate as FactoryInstantiate, query as FactoryQuery,
        reply as FactoryReply,
    };

    use cw20_base::contract::{
        execute as Cw20Execute, instantiate as Cw20Instantiate, query as Cw20Query,
    };

    use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

    use cw721_base::entry::{
        execute as Cw721Execute, instantiate as Cw721Instantiate, query as Cw721Query,
    };

    use cw721_base::msg::InstantiateMsg as Cw721InstantiateMsg;

    use campaign::contract::{execute as Execute, instantiate as Instantiate, query as Query};

    use crate::msg::InstantiateMsg as FactoryInstantiateMsg;

    pub const ADMIN: &str = "aura1000000000000000000000000000000000admin";
    pub const USER_1: &str = "aura1000000000000000000000000000000000user1";
    pub const USER_2: &str = "aura1000000000000000000000000000000000user2";

    pub struct ContractInfo {
        pub contract_addr: String,
        pub contract_code_id: u64,
    }

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN), vec![])
                .unwrap();
        })
    }

    // factory contract
    fn factory_contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(FactoryExecute, FactoryInstantiate, FactoryQuery)
            .with_reply(FactoryReply);
        Box::new(contract)
    }

    // campaign contract
    fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(Execute, Instantiate, Query);
        Box::new(contract)
    }

    // token contract
    // create instantiate message for contract
    fn token_contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(Cw20Execute, Cw20Instantiate, Cw20Query);
        Box::new(contract)
    }

    // collection contract
    fn collection_contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(Cw721Execute, Cw721Instantiate, Cw721Query);
        Box::new(contract)
    }

    pub fn instantiate_contracts() -> (App, Vec<ContractInfo>) {
        // Create a new app instance
        let mut app = mock_app();
        // Create a vector to store all contract info ([factory - [0])
        let mut contracts: Vec<ContractInfo> = Vec::new();

        // store code of all contracts to the app and get the code ids
        let factory_contract_code_id = app.store_code(factory_contract_template());
        let token_contract_code_id = app.store_code(token_contract_template());
        let collection_contract_code_id = app.store_code(collection_contract_template());

        // factory contract
        // create instantiate message for contract
        let factory_instantiate_msg = FactoryInstantiateMsg {
            campaign_code_id: app.store_code(contract_template()),
        };

        // factory instantiate contract
        let factory_contract_addr = app
            .instantiate_contract(
                factory_contract_code_id,
                Addr::unchecked(ADMIN),
                &factory_instantiate_msg,
                &[],
                "test instantiate contract",
                None,
            )
            .unwrap();

        // add contract info to vector
        contracts.push(ContractInfo {
            contract_addr: factory_contract_addr.to_string(),
            contract_code_id: factory_contract_code_id,
        });

        // token contract
        // create instantiate message for contract
        let lp_token_instantiate_msg = Cw20InstantiateMsg {
            name: "Token".to_string(),
            symbol: "TTT".to_string(),
            decimals: 3,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: ADMIN.to_string(),
                cap: None,
            }),
            marketing: None,
        };

        // token instantiate contract
        let token_contract_addr = app
            .instantiate_contract(
                token_contract_code_id,
                Addr::unchecked(ADMIN),
                &lp_token_instantiate_msg,
                &[],
                "test instantiate contract",
                None,
            )
            .unwrap();

        // add contract info to the vector
        contracts.push(ContractInfo {
            contract_addr: token_contract_addr.to_string(),
            contract_code_id: token_contract_code_id,
        });

        // collection contract
        // create instantiate message for contract
        let collection_instantiate_msg = Cw721InstantiateMsg {
            name: "LP Token".to_string(),
            symbol: "LPTT".to_string(),
            minter: ADMIN.to_string(),
        };

        // token instantiate contract
        let collection_contract_addr = app
            .instantiate_contract(
                collection_contract_code_id,
                Addr::unchecked(ADMIN),
                &collection_instantiate_msg,
                &[],
                "test instantiate contract",
                None,
            )
            .unwrap();

        // add contract info to the vector
        contracts.push(ContractInfo {
            contract_addr: collection_contract_addr.to_string(),
            contract_code_id: collection_contract_code_id,
        });

        (app, contracts)
    }
}
