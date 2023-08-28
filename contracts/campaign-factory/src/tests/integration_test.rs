#![cfg(test)]
mod tests {
    const MOCK_1000_TOKEN_AMOUNT: u128 = 1_000_000;
    // 1. create token contract, collection contract
    // 2. create factory contract
    // 3. create campaign by factory contract
    // 4. add reward by token contract
    // 5. stake nft by collection contract
    // 6. claim reward
    // 7. unstake nft
    // 8. withdraw remaining reward
    mod execute_proper_operation {
        use crate::{
            msg::QueryMsg,
            state::{FactoryCampaign, Metadata},
            tests::{
                env_setup::env::{instantiate_contracts, ADMIN, USER_1, USER_2},
                integration_test::tests::MOCK_1000_TOKEN_AMOUNT,
            },
        };
        use campaign::state::{
            AssetToken, CampaignInfoResult, CampaignInfoUpdate, LockupTerm, NftInfo, NftStake,
            StakedInfoResult, TokenInfo,
        };
        use campaign::{
            msg::{ExecuteMsg as CampaignExecuteMsg, QueryMsg as CampaignQueryMsg},
            utils::{add_reward, calc_reward_in_time, sub_reward},
        };
        use cosmwasm_std::{Addr, BlockInfo, Empty, Uint128};
        use cw20::{BalanceResponse, Cw20ExecuteMsg};
        use cw721_base::MintMsg as Cw721MintMsg;
        use cw_multi_test::Executor;

        pub type Extension = Option<Metadata>;
        pub type Cw721ExecuteMsg = cw721_base::ExecuteMsg<Extension, Empty>;

        //         -------------- proper operation ------------------
        // - ADMIN create campaign contract by factory contract
        // - add 1000.000 reward token to campaign by ADMIN
        // - with end time 100s -> reward_per_second = 10.000 token
        // - increase 20s to make active campaign
        // - stake nft token_id 1 with lockup_term = 10s, percent = 30% to campaign by USER_1
        // - token_id 1 -> has time stake: s20 -> s30
        // - increase simulation time more 1s
        // - stake token_id 2 with lockup_term = 10s, percent = 30% -> has time staking: s21 -> s31 -> calculate pending reward token_id 1
        // 	- token_id 1 pending_reward = 1(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 1 (nft_count) = 3.000 token
        // - increase simulation time more 6s
        // 	- token_id 1 pending_reward = 3.000 + 6(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 12000
        // 	- token_id 2 pending_reward = 6(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 9000
        // - USER_1 reward_debt = 12.000 + 9.000 = 21.000, reward_claimed = 0
        // - USER_1 claim reward: 21.000
        // 	- token_id 1 pending_reward = 0
        // 	- token_id 2 pending_reward = 0
        // 	- USER_1 reward_debt = 0, reward_claimed = 21.000
        // 	- total_reward = 1000.000 - 21.000 = 979.000
        // - increase simulation time more 3s
        // 	- USER_1 un_stake nft 1
        // 		- calc token_id 1: pending_reward = 0 + 3(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 4.500	move reward to USER_1, remove token_id 1 from USER_1
        // 		- token_id 2:  pending_reward = 0 + 3(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 4.500
        // 		- USER_1 reward_debt = 4.500(token_id 1 transerfered) + 4.500 (token_id 2) = 9.000, reward_claimed = 21.000
        // - total_pending_reward = 4.500(token_id 2) + 4.500(reward_debt USER_1) = 9.000
        // - increase simulation time more 80s -> ended campaign
        // - USER_1:
        // 	- token_id 2: pending_reward = 4.500 + 1(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 1 (nft_count) = 7.500
        // 	- USER_1 reward_debt = 4.500(token_id 1 transerfered) + 7.500(token_id 2) = 12.000, reward_claimed = 21.000
        // 	- total_pending_reward = 7.500(token_id 2) + 4.500(reward_debt USER_1) = 12.000
        // - withdraw remaining reward by ADMIN
        // 	- total_pending_reward = 12.000
        // 	- withdraw_reward = 979.000 - 12.000(total_pending_reward) = 967.000
        //  - ADMIN token = 967.000
        // 	- Campaign token = 12.000
        #[test]
        fn proper_operation() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();

            // get factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get lp token contract
            let token_contract = &contracts[1].contract_addr;
            // get collection contract
            let collection_contract = &contracts[2].contract_addr;

            // Mint 1000 tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // mint 5 nft with token_id = 1..5 to USER_1
            for id in 1..5 {
                // mint nft
                let mint_nft_msg = Cw721MintMsg {
                    token_id: id.to_string(),
                    owner: USER_1.to_string(),
                    token_uri: Some(
                        "https://starships.example.com/Starship/Enterprise.json".into(),
                    ),
                    extension: Some(Metadata {
                        description: Some("Spaceship with Warp Drive".into()),
                        name: Some("Starship USS Enterprise".to_string()),
                        ..Metadata::default()
                    }),
                };

                let exec_msg = Cw721ExecuteMsg::Mint(mint_nft_msg.clone());

                let response_mint_nft = app.execute_contract(
                    Addr::unchecked(ADMIN.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &exec_msg,
                    &[],
                );

                assert!(response_mint_nft.is_ok());
            }

            // mint 5 nft with token_id = 6..10 to USER_2
            for id in 6..10 {
                // mint nft
                let mint_nft_msg = Cw721MintMsg {
                    token_id: id.to_string(),
                    owner: USER_2.to_string(),
                    token_uri: Some(
                        "https://starships.example.com/Starship/Enterprise.json".into(),
                    ),
                    extension: Some(Metadata {
                        description: Some("Spaceship with Warp Drive".into()),
                        name: Some("Starship USS Enterprise".to_string()),
                        ..Metadata::default()
                    }),
                };

                let exec_msg = Cw721ExecuteMsg::Mint(mint_nft_msg.clone());

                let response_mint_nft = app.execute_contract(
                    Addr::unchecked(ADMIN.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &exec_msg,
                    &[],
                );

                assert!(response_mint_nft.is_ok());
            }

            // Approve all nft by USER_1
            for id in 1..5 {
                // Approve nft to campaign contract
                let approve_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::Approve {
                    spender: "contract3".to_string(), // Campaign Contract
                    token_id: id.to_string(),
                    expires: None,
                };

                // Execute approve nft
                let response = app.execute_contract(
                    Addr::unchecked(USER_1.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &approve_msg,
                    &[],
                );
                assert!(response.is_ok());
            }

            // Approve all nft by USER_2
            for id in 6..10 {
                // Approve nft to campaign contract
                let approve_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::Approve {
                    spender: "contract3".to_string(), // Campaign Contract
                    token_id: id.to_string(),
                    expires: None,
                };

                // Execute approve nft
                let response = app.execute_contract(
                    Addr::unchecked(USER_2.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &approve_msg,
                    &[],
                );
                assert!(response.is_ok());
            }

            // query balance of ADMIN in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();
            // It should be 1000 token as minting happened
            assert_eq!(balance.balance, Uint128::from(MOCK_1000_TOKEN_AMOUNT));

            // token info
            let token_info = TokenInfo::Token {
                contract_addr: token_contract.to_string(),
            };

            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 110,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );

            assert!(response_create_campaign.is_ok());

            // query campaign contract address
            let campaign_info: FactoryCampaign = app
                .wrap()
                .query_wasm_smart(
                    factory_contract.clone(),
                    &crate::msg::QueryMsg::Campaign { campaign_id: 1u64 },
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                FactoryCampaign {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_addr: Addr::unchecked("contract3"),
                    reward_token: TokenInfo::Token {
                        contract_addr: token_contract.to_string()
                    },
                    allowed_collection: Addr::unchecked(collection_contract)
                }
            );

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign name".to_string(),
                    campaign_description: "campaign name".to_string(),
                    limit_per_staker: 2,
                    reward_token_info: AssetToken {
                        info: token_info.clone(),
                        amount: Uint128::zero(),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 0,
                    total_reward_claimed: Uint128::zero(),
                    total_reward: Uint128::zero(),
                    reward_per_second: Uint128::zero(),
                    time_calc_nft: 0,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            // update campaign
            let update_campaign_msg = CampaignExecuteMsg::UpdateCampaign {
                campaign_info_update: CampaignInfoUpdate {
                    campaign_name: None,
                    campaign_image: Some("campaign image".to_string()),
                    campaign_description: Some("campaign description".to_string()),
                    limit_per_staker: Some(4),
                    lockup_term: None,
                    start_time: None,
                    end_time: None,
                },
            };

            // Execute update campaign
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &update_campaign_msg,
                &[],
            );

            assert!(response.is_ok());

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign image".to_string(),
                    campaign_description: "campaign description".to_string(),
                    limit_per_staker: 4,
                    reward_token_info: AssetToken {
                        info: token_info.clone(),
                        amount: Uint128::zero(),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 0,
                    total_reward_claimed: Uint128::zero(),
                    total_reward: Uint128::zero(),
                    reward_per_second: Uint128::zero(),
                    time_calc_nft: 0,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            // query all campaigns in factory contract
            let campaigns: Vec<FactoryCampaign> = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked(factory_contract.clone()),
                    &QueryMsg::Campaigns {
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap();

            // TODO: contract3 unknown ?
            assert_eq!(
                campaigns,
                vec![FactoryCampaign {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_addr: Addr::unchecked("contract3"),
                    reward_token: TokenInfo::Token {
                        contract_addr: token_contract.to_string()
                    },
                    allowed_collection: Addr::unchecked(collection_contract)
                }]
            );

            // Approve cw20 token to campaign contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Campaign Contract
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // add reward token
            let add_reward_balance_msg = CampaignExecuteMsg::AddRewardToken {
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[],
            );

            assert!(response.is_ok());

            // check reward token in campaign
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::CampaignInfo {})
                .unwrap();

            assert_eq!(
                Uint128::from(MOCK_1000_TOKEN_AMOUNT),
                campaign_info.reward_token_info.amount
            );

            // query balance of ADMIN in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();

            // It should be 0 token as deposit happened
            assert_eq!(balance.balance, Uint128::zero());

            // query balance of campaign contract in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: "contract3".to_string(),
                    },
                )
                .unwrap();

            // It should be MOCK_1000_TOKEN_AMOUNT token as deposit happened
            assert_eq!(balance.balance, Uint128::from(MOCK_1000_TOKEN_AMOUNT));

            // increase 20 second to make active campaign
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(20),
                height: app.block_info().height + 20,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 1
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "1".to_string(),
                    lockup_term: 10,
                }],
            };
            let start_time_1 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // get nft info
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "1".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "1".to_string(),
                    owner: Addr::unchecked(USER_1.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_1,
                    end_time: start_time_1 + 10
                }
            );

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![NftInfo {
                        token_id: "1".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(0u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128)
                        },
                        is_end_reward: false,
                        start_time: start_time_1,
                        end_time: start_time_1 + 10
                    }],
                    reward_debt: Uint128::zero(),
                    reward_claimed: Uint128::zero()
                }
            );

            // change block time increase 1 second to next stake nft 2
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(1),
                height: app.block_info().height + 1,
                chain_id: app.block_info().chain_id,
            });

            // get nft info token_id 1
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "1".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "1".to_string(),
                    owner: Addr::unchecked(USER_1.to_string()),
                    pending_reward: Uint128::from(3000u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_1,
                    end_time: start_time_1 + 10
                }
            );

            // stake nft token_id 2
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "2".to_string(),
                    lockup_term: 10,
                }],
            };
            let start_time_2 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // get nft info token_id 1
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "1".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "1".to_string(),
                    owner: Addr::unchecked(USER_1.to_string()),
                    pending_reward: Uint128::from(3000u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_1,
                    end_time: start_time_1 + 10
                }
            );

            // get nft info token_id 2
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "2".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "2".to_string(),
                    owner: Addr::unchecked(USER_1.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_2,
                    end_time: start_time_2 + 10
                }
            );

            // increase 6 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(6),
                height: app.block_info().height + 6,
                chain_id: app.block_info().chain_id,
            });

            // get staker info by USER_1
            let staker_info: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staker_info,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(12000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128),
                            },
                            is_end_reward: false,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(9000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128),
                            },
                            is_end_reward: false,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        },
                    ],
                    reward_debt: Uint128::from(21000u128),
                    reward_claimed: Uint128::zero()
                }
            );

            // USER_1 claim reward msg
            let claim_reward_msg = CampaignExecuteMsg::ClaimReward {
                amount: Uint128::from(21000u128),
            };

            // Execute claim reward
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &claim_reward_msg,
                &[],
            );

            assert!(response.is_ok());

            // get staker info
            let staker_info: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staker_info,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128),
                            },
                            is_end_reward: false,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128),
                            },
                            is_end_reward: false,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        },
                    ],
                    reward_debt: Uint128::from(0u128),
                    reward_claimed: Uint128::from(21000u128),
                }
            );

            // increase 3 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(3),
                height: app.block_info().height + 3,
                chain_id: app.block_info().chain_id,
            });

            // USER_1 un stake nft msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "1".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // get staker info USER_1
            let staker_info: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staker_info,
                StakedInfoResult {
                    nfts: vec![NftInfo {
                        token_id: "2".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(4500u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128),
                        },
                        is_end_reward: false,
                        start_time: start_time_2,
                        end_time: start_time_2 + 10
                    },],
                    reward_debt: Uint128::from(9000u128),
                    reward_claimed: Uint128::from(21000u128),
                }
            );

            // get staker total pending reward
            let total_pending_reward: Uint128 = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::TotalPendingReward {})
                .unwrap();

            // token_id 2 = 4500, reward_debt USER_1 = 4500(token_id 1 unstake transerfered)
            assert_eq!(total_pending_reward, Uint128::from(9000u128));

            // increase 80 second to ended campaign
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(80),
                height: app.block_info().height + 80,
                chain_id: app.block_info().chain_id,
            });

            // get staker info
            let staker_info: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staker_info,
                StakedInfoResult {
                    nfts: vec![NftInfo {
                        token_id: "2".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(7500u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128),
                        },
                        is_end_reward: true,
                        start_time: start_time_2,
                        end_time: start_time_2 + 10
                    },],
                    reward_debt: Uint128::from(12000u128),
                    reward_claimed: Uint128::from(21000u128)
                }
            );

            // get total pending reward
            let total_pending_reward: Uint128 = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::TotalPendingReward {})
                .unwrap();

            // token_id 2 = 7500, reward_debt USER_1 = 4500(token_id 1 unstake transerfered)
            assert_eq!(total_pending_reward, Uint128::from(12000u128));

            // withdraw remaining reward msg = 979000 - 12000
            let withdraw_reward_msg = CampaignExecuteMsg::WithdrawReward {};

            // Execute withdraw remaining reward
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_reward_msg,
                &[],
            );

            assert!(response.is_ok());

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign image".to_string(),
                    campaign_description: "campaign description".to_string(),
                    limit_per_staker: 4,
                    reward_token_info: AssetToken {
                        info: token_info,
                        amount: Uint128::from(12000u128),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 1,
                    total_reward_claimed: Uint128::from(21000u128),
                    total_reward: Uint128::from(1000000u128),
                    reward_per_second: Uint128::from(10000u128),
                    time_calc_nft: current_block_time + 110,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            // get staker info
            let staker_info: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staker_info,
                StakedInfoResult {
                    nfts: vec![NftInfo {
                        token_id: "2".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(7500u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128),
                        },
                        is_end_reward: true,
                        start_time: start_time_2,
                        end_time: start_time_2 + 10
                    },],
                    reward_debt: Uint128::from(12000u128),
                    reward_claimed: Uint128::from(21000u128),
                }
            );

            // get staker total pending reward
            let total_pending_reward: Uint128 = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::TotalPendingReward {})
                .unwrap();

            assert_eq!(total_pending_reward, Uint128::from(12000u128));
        }

        //         -------------- proper operation with multiple users ------------------
        // - ADMIN create campaign contract by factory contract
        // - add 1000.000 reward token to campaign by ADMIN
        // - with end time 10s -> 110s -> reward_per_second = 10.000 token
        // -----------Phrase 0: calculate reward for staking nft in active campaign ------------
        // - increase 20s to make active campaign: 20s
        // 	- stake nft token_id 1 with lockup_term = 10s, percent = 30% to campaign by USER_1 -> token_id 1 -> has time stake: s20 -> s30
        // 	- stake nft token_id 2 with lockup_term = 10s, percent = 30% to campaign by USER_1 -> token_id 2 -> has time stake: s20 -> s30
        // - increase simulation time more 5s: 25s
        // - USER_2 stake token_id 6 with lockup_term = 30s, percent = 70% -> has time stake: s25 -> s55 -> calculate pending reward token_id 1,2
        // 	- token_id 1 pending_reward = 5(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 7.500 token
        // 	- token_id 2 pending_reward = 5(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 7.500 token
        // - increase simulation time more 5s: 30s
        // 	- USER_1 stake token_id 3 with lockup_term = 30s, percent = 70% -> has time stake: s30 -> s60 -> calculate pending reward token_id 1,2,6
        // 	- token_id 1 pending_reward = 7.500 + 5(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 15.000 -> end
        // 	- token_id 2 pending_reward = 7.500 + 5(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 2 (nft_count) = 15.000 -> end
        // 	- token_id 6 pending_reward = 5(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 1 (nft_count) = 35.000
        // - increase simulation time more 5s: 35s
        // 	- token_id 6 pending_reward = 35.000 + 5(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 2 (nft_count) = 52.500
        // 	- token_id 3 pending_reward = 5(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 2 (nft_count) = 17.500
        // 	- USER_1 reward_debt = 15.000(token_id 1) + 15.000(token_id 2) + 17.500(token_id 3) = 47.500, reward_claimed = 0
        // 	- USER_2 reward_debt = 52.500(token_id 6)= 52.500, reward_claimed = 0, nfts = 6
        // 	- USER_1 claim reward: 47.500
        // 		- token_id 1 pending_reward = 0
        // 		- token_id 2 pending_reward = 0
        // 		- token_id 3 pending_reward = 0
        // 		- USER_1 reward_debt = 0, reward_claimed = 47.500, nfts = 1,2,3
        // -----------Phrase 1: calc reward for staking nft but time out campaign ------------
        // - increase simulation time more 55s: 90s
        // 	- USER_2 stake token_id 7 with lockup_term = 30s, percent = 70% -> has time stake: s90 -> s120 -> calculate pending reward token_id 6,3
        // 	- token_id 6 pending_reward = 52.500 + 20(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 2 (nft_count) = 122.500 -> end
        // 	- token_id 3 pending_reward = 20(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 2 (nft_count) + 5(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 1 (nft_count) = 105.000 -> end
        // 	- USER_2 stake token_id 8 with lockup_term = 10s, percent = 30% -> has time stake: s90 -> s100 -> calculate pending reward token_id 7
        // 	- token_id 7 pending_reward = 0
        // - increase simulation time more 10s: 100s
        // 	- USER_1 stake token_id 4 with lockup_term = 30s, percent = 70% -> has time stake: s100 -> s130 -> calculate pending reward token_id 7,8
        // 	- token_id 7 pending_reward = 10(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 1 (nft_count) = 70.000
        // 	- token_id 8 pending_reward = 10(s) * 10.000(reward_per_second) * 30 / 100 (percent_lockup_term) / 1 (nft_count) = 30.000 -> end
        // - increase simulation time more 20s: 120s
        // 	- we will calculate reward to 110s(campaign ended)  calculate pending reward token_id 7,4
        // 	- token_id 7 pending_reward = 70.000 + 10(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 2 (nft_count) = 105.000 -> end
        // 	- token_id 4 pending_reward = 10(s) * 10.000(reward_per_second) * 70 / 100 (percent_lockup_term) / 2 (nft_count) = 35.000 -> end
        // 	- USER_1 reward_debt = 105.000(token_id 3) + 35.000(token_id 4) = 140.000, reward_claimed = 47.500, nfts = 1,2,3,4
        // 	- USER_2 reward_debt = 122.500(token_id 6) + 30.000(token_id 8) + 105.000(token_id 7) = 257.500, reward_claimed = 0, nfts = 6,7,8
        // - unstake nfts = 1,2,3,4,5,6,7,8
        // 	- USER_1 reward_debt = 140.000, reward_claimed = 47.500, nfts = []
        // 	- USER_2 reward_debt = 257.500, reward_claimed = 0, nfts = []
        // 	- total_pending_reward = 140.000 + 257.500 = 397.500
        // 	- withdraw remaining reward = 1000.000(total) - 47.500(USER_1 claimed) - 397.500(pending) = 555.000
        // 	- ADMIN token = 555.000
        // 	- Campaign token = 397.500
        #[test]
        fn proper_operation_with_multiple_users() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();

            // get factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get lp token contract
            let token_contract = &contracts[1].contract_addr;
            // get collection contract
            let collection_contract = &contracts[2].contract_addr;

            // Mint 1000 tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // mint 5 nft with token_id = 1..5 to USER_1
            for id in 1..5 {
                // mint nft
                let mint_nft_msg = Cw721MintMsg {
                    token_id: id.to_string(),
                    owner: USER_1.to_string(),
                    token_uri: Some(
                        "https://starships.example.com/Starship/Enterprise.json".into(),
                    ),
                    extension: Some(Metadata {
                        description: Some("Spaceship with Warp Drive".into()),
                        name: Some("Starship USS Enterprise".to_string()),
                        ..Metadata::default()
                    }),
                };

                let exec_msg = Cw721ExecuteMsg::Mint(mint_nft_msg.clone());

                let response_mint_nft = app.execute_contract(
                    Addr::unchecked(ADMIN.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &exec_msg,
                    &[],
                );

                assert!(response_mint_nft.is_ok());
            }

            // mint 5 nft with token_id = 6..10 to USER_2
            for id in 6..10 {
                // mint nft
                let mint_nft_msg = Cw721MintMsg {
                    token_id: id.to_string(),
                    owner: USER_2.to_string(),
                    token_uri: Some(
                        "https://starships.example.com/Starship/Enterprise.json".into(),
                    ),
                    extension: Some(Metadata {
                        description: Some("Spaceship with Warp Drive".into()),
                        name: Some("Starship USS Enterprise".to_string()),
                        ..Metadata::default()
                    }),
                };

                let exec_msg = Cw721ExecuteMsg::Mint(mint_nft_msg.clone());

                let response_mint_nft = app.execute_contract(
                    Addr::unchecked(ADMIN.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &exec_msg,
                    &[],
                );

                assert!(response_mint_nft.is_ok());
            }

            // Approve all nft by USER_1
            for id in 1..5 {
                // Approve nft to campaign contract
                let approve_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::Approve {
                    spender: "contract3".to_string(), // Campaign Contract
                    token_id: id.to_string(),
                    expires: None,
                };

                // Execute approve nft
                let response = app.execute_contract(
                    Addr::unchecked(USER_1.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &approve_msg,
                    &[],
                );
                assert!(response.is_ok());
            }

            // Approve all nft by USER_2
            for id in 6..10 {
                // Approve nft to campaign contract
                let approve_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::Approve {
                    spender: "contract3".to_string(), // Campaign Contract
                    token_id: id.to_string(),
                    expires: None,
                };

                // Execute approve nft
                let response = app.execute_contract(
                    Addr::unchecked(USER_2.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &approve_msg,
                    &[],
                );
                assert!(response.is_ok());
            }

            // query balance of ADMIN in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();
            // It should be 1000 lp token as minting happened
            assert_eq!(balance.balance, Uint128::from(MOCK_1000_TOKEN_AMOUNT));

            // token info
            let token_info = TokenInfo::Token {
                contract_addr: token_contract.to_string(),
            };

            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 110,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );

            assert!(response_create_campaign.is_ok());

            // query campaign contract address
            let campaign_info: FactoryCampaign = app
                .wrap()
                .query_wasm_smart(
                    factory_contract.clone(),
                    &crate::msg::QueryMsg::Campaign { campaign_id: 1u64 },
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                FactoryCampaign {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_addr: Addr::unchecked("contract3"),
                    reward_token: TokenInfo::Token {
                        contract_addr: token_contract.to_string()
                    },
                    allowed_collection: Addr::unchecked(collection_contract)
                }
            );

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign name".to_string(),
                    campaign_description: "campaign name".to_string(),
                    limit_per_staker: 2,
                    reward_token_info: AssetToken {
                        info: token_info.clone(),
                        amount: Uint128::zero(),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 0,
                    total_reward_claimed: Uint128::zero(),
                    total_reward: Uint128::zero(),
                    reward_per_second: Uint128::zero(),
                    time_calc_nft: 0,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            // update campaign
            let update_campaign_msg = CampaignExecuteMsg::UpdateCampaign {
                campaign_info_update: CampaignInfoUpdate {
                    campaign_name: None,
                    campaign_image: Some("campaign image".to_string()),
                    campaign_description: Some("campaign description".to_string()),
                    limit_per_staker: Some(4),
                    lockup_term: None,
                    start_time: None,
                    end_time: None,
                },
            };

            // Execute update campaign
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &update_campaign_msg,
                &[],
            );

            assert!(response.is_ok());

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign image".to_string(),
                    campaign_description: "campaign description".to_string(),
                    limit_per_staker: 4,
                    reward_token_info: AssetToken {
                        info: token_info.clone(),
                        amount: Uint128::zero(),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 0,
                    total_reward_claimed: Uint128::zero(),
                    total_reward: Uint128::zero(),
                    reward_per_second: Uint128::zero(),
                    time_calc_nft: 0,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            // query all campaigns in factory contract
            let campaigns: Vec<FactoryCampaign> = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked(factory_contract.clone()),
                    &QueryMsg::Campaigns {
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap();

            // TODO: contract3 unknown ?
            assert_eq!(
                campaigns,
                vec![FactoryCampaign {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_addr: Addr::unchecked("contract3"),
                    reward_token: TokenInfo::Token {
                        contract_addr: token_contract.to_string()
                    },
                    allowed_collection: Addr::unchecked(collection_contract)
                }]
            );

            // Approve cw20 token to campaign contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Campaign Contract
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // add reward token
            let add_reward_balance_msg = CampaignExecuteMsg::AddRewardToken {
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[],
            );

            assert!(response.is_ok());

            // check reward token in campaign
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::CampaignInfo {})
                .unwrap();

            assert_eq!(
                Uint128::from(MOCK_1000_TOKEN_AMOUNT),
                campaign_info.reward_token_info.amount
            );

            // query balance of ADMIN in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();

            // It should be 0 token as deposit happened
            assert_eq!(balance.balance, Uint128::zero());

            // query balance of campaign contract in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: "contract3".to_string(),
                    },
                )
                .unwrap();

            // It should be MOCK_1000_TOKEN_AMOUNT token as deposit happened
            assert_eq!(balance.balance, Uint128::from(MOCK_1000_TOKEN_AMOUNT));

            // increase 20 second to make active campaign
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(20),
                height: app.block_info().height + 20,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 1
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "1".to_string(),
                    lockup_term: 10,
                }],
            };
            let start_time_1 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // stake nft token_id 2
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "2".to_string(),
                    lockup_term: 10,
                }],
            };
            let start_time_2 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // get nft info
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "1".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "1".to_string(),
                    owner: Addr::unchecked(USER_1.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_1,
                    end_time: start_time_1 + 10
                }
            );

            // get nft info id 2
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "2".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "2".to_string(),
                    owner: Addr::unchecked(USER_1.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_2,
                    end_time: start_time_2 + 10
                }
            );

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        }
                    ],
                    reward_debt: Uint128::zero(),
                    reward_claimed: Uint128::zero()
                },
            );

            // increase 5 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 6
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "6".to_string(),
                    lockup_term: 30,
                }],
            };
            let start_time_6 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(7500u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(7500u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        }
                    ],
                    reward_debt: Uint128::from(15000u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_2.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![NftInfo {
                        token_id: "6".to_string(),
                        owner: Addr::unchecked(USER_2.to_string()),
                        pending_reward: Uint128::from(0u128),
                        lockup_term: LockupTerm {
                            value: 30,
                            percent: Uint128::from(70u128)
                        },
                        is_end_reward: false,
                        start_time: start_time_6,
                        end_time: start_time_6 + 30
                    }],
                    reward_debt: Uint128::zero(),
                    reward_claimed: Uint128::zero()
                },
            );

            // increase 5 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 3
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "3".to_string(),
                    lockup_term: 30,
                }],
            };
            let start_time_3 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(15000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(15000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 30
                        }
                    ],
                    reward_debt: Uint128::from(30000u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_2.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![NftInfo {
                        token_id: "6".to_string(),
                        owner: Addr::unchecked(USER_2.to_string()),
                        pending_reward: Uint128::from(35000u128),
                        lockup_term: LockupTerm {
                            value: 30,
                            percent: Uint128::from(70u128)
                        },
                        is_end_reward: false,
                        start_time: start_time_6,
                        end_time: start_time_6 + 30
                    }],
                    reward_debt: Uint128::from(35000u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // increase 5 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(15000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(15000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(17500u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 30
                        }
                    ],
                    reward_debt: Uint128::from(47500u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // USER_1 claim reward msg
            let claim_reward_msg = CampaignExecuteMsg::ClaimReward {
                amount: Uint128::from(47500u128),
            };

            // Execute claim reward
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &claim_reward_msg,
                &[],
            );

            assert!(response.is_ok());

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 30
                        }
                    ],
                    reward_debt: Uint128::from(0u128),
                    reward_claimed: Uint128::from(47500u128)
                },
            );

            // increase 55 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(55),
                height: app.block_info().height + 55,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 7
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "7".to_string(),
                    lockup_term: 30,
                }],
            };
            let start_time_7 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // get nft info id 7
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "7".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "7".to_string(),
                    owner: Addr::unchecked(USER_2.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 30,
                        percent: Uint128::from(70u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_7,
                    end_time: start_time_7 + 30
                }
            );

            assert_eq!(current_block_time + 90, start_time_7);

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign image".to_string(),
                    campaign_description: "campaign description".to_string(),
                    limit_per_staker: 4,
                    reward_token_info: AssetToken {
                        info: token_info.clone(),
                        amount: Uint128::from(952500u128),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 5,
                    total_reward_claimed: Uint128::from(47500u128),
                    total_reward: Uint128::from(1000000u128),
                    reward_per_second: Uint128::from(10000u128),
                    time_calc_nft: current_block_time + 90,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            // query token_ids
            let token_ids: Vec<String> = app
                .wrap()
                .query_wasm_smart(Addr::unchecked("contract3"), &CampaignQueryMsg::TokenIds {})
                .unwrap();

            assert_eq!(token_ids, vec!["1", "2", "6", "3", "7"]);

            // stake nft token_id 8
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "8".to_string(),
                    lockup_term: 10,
                }],
            };
            let start_time_8 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign image".to_string(),
                    campaign_description: "campaign description".to_string(),
                    limit_per_staker: 4,
                    reward_token_info: AssetToken {
                        info: token_info.clone(),
                        amount: Uint128::from(952500u128),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 6,
                    total_reward_claimed: Uint128::from(47500u128),
                    total_reward: Uint128::from(1000000u128),
                    reward_per_second: Uint128::from(10000u128),
                    time_calc_nft: current_block_time + 90,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );

            assert_eq!(campaign_info.time_calc_nft, start_time_8);

            // get nft info id 7
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::Nft {
                        token_id: "7".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "7".to_string(),
                    owner: Addr::unchecked(USER_2.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 30,
                        percent: Uint128::from(70u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_7,
                    end_time: start_time_7 + 30
                }
            );

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_2.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "8".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_8,
                            end_time: start_time_8 + 10
                        },
                        NftInfo {
                            token_id: "6".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(122500u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_6,
                            end_time: start_time_6 + 30
                        },
                        NftInfo {
                            token_id: "7".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_7,
                            end_time: start_time_7 + 30
                        }
                    ],
                    reward_debt: Uint128::from(122500u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // get nft info id 7
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "7".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "7".to_string(),
                    owner: Addr::unchecked(USER_2.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 30,
                        percent: Uint128::from(70u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_7,
                    end_time: start_time_7 + 30
                }
            );

            // get nft info id 8
            let nft_info: NftInfo = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftInfo {
                        token_id: "8".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(
                nft_info,
                NftInfo {
                    token_id: "8".to_string(),
                    owner: Addr::unchecked(USER_2.to_string()),
                    pending_reward: Uint128::from(0u128),
                    lockup_term: LockupTerm {
                        value: 10,
                        percent: Uint128::from(30u128)
                    },
                    is_end_reward: false,
                    start_time: start_time_8,
                    end_time: start_time_8 + 10
                }
            );

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_2.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "8".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_8,
                            end_time: start_time_8 + 10
                        },
                        NftInfo {
                            token_id: "6".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(122500u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_6,
                            end_time: start_time_6 + 30
                        },
                        NftInfo {
                            token_id: "7".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_7,
                            end_time: start_time_7 + 30
                        }
                    ],
                    reward_debt: Uint128::from(122500u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // increase 10 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(10),
                height: app.block_info().height + 10,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 4
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "4".to_string(),
                    lockup_term: 30,
                }],
            };
            let start_time_4 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // increase 20 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(20),
                height: app.block_info().height + 20,
                chain_id: app.block_info().chain_id,
            });

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_2,
                            end_time: start_time_2 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(105000u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_3,
                            end_time: start_time_3 + 30
                        },
                        NftInfo {
                            token_id: "4".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(35000u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_4,
                            end_time: start_time_4 + 30
                        }
                    ],
                    reward_debt: Uint128::from(140000u128),
                    reward_claimed: Uint128::from(47500u128)
                },
            );

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_2.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "8".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(30000u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_8,
                            end_time: start_time_8 + 10
                        },
                        NftInfo {
                            token_id: "6".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(122500u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_6,
                            end_time: start_time_6 + 30
                        },
                        NftInfo {
                            token_id: "7".to_string(),
                            owner: Addr::unchecked(USER_2.to_string()),
                            pending_reward: Uint128::from(105000u128),
                            lockup_term: LockupTerm {
                                value: 30,
                                percent: Uint128::from(70u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_7,
                            end_time: start_time_7 + 30
                        }
                    ],
                    reward_debt: Uint128::from(257500u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // USER_1 un stake nft 1 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "1".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_1 un stake nft 2 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "2".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_1 un stake nft 3 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "3".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_1 un stake nft 4 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "4".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_2 un stake nft 6 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "6".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_2 un stake nft 6 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "7".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // USER_2 un stake nft 6 msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "8".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // nft staked with USER_1
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![],
                    reward_debt: Uint128::from(140000u128),
                    reward_claimed: Uint128::from(47500u128)
                },
            );

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_2.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![],
                    reward_debt: Uint128::from(257500u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // get staker total pending reward
            let total_pending_reward: Uint128 = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::TotalPendingReward {})
                .unwrap();

            // USER_1 = 140000, USER_2 = 257500
            assert_eq!(total_pending_reward, Uint128::from(397500u128));

            // withdraw remaining reward msg = 1000000 - 47500 - 397500 = 555000
            let withdraw_reward_msg = CampaignExecuteMsg::WithdrawReward {};

            // Execute withdraw remaining reward
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_reward_msg,
                &[],
            );

            assert!(response.is_ok());

            // query balance of ADMIN in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();

            // It should be 555000 token as withdraw happened
            assert_eq!(balance.balance, Uint128::from(555000u128));

            // query balance of campaign contract in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: "contract3".to_string(),
                    },
                )
                .unwrap();

            assert_eq!(balance.balance, Uint128::from(397500u128));

            // query campaign contract address
            let campaign_info: CampaignInfoResult = app
                .wrap()
                .query_wasm_smart(
                    Addr::unchecked("contract3"),
                    &CampaignQueryMsg::CampaignInfo {},
                )
                .unwrap();

            // assert campaign info
            assert_eq!(
                campaign_info,
                CampaignInfoResult {
                    owner: Addr::unchecked(ADMIN.to_string()),
                    campaign_name: "campaign name".to_string(),
                    campaign_image: "campaign image".to_string(),
                    campaign_description: "campaign description".to_string(),
                    limit_per_staker: 4,
                    reward_token_info: AssetToken {
                        info: token_info,
                        amount: Uint128::from(397500u128),
                    },
                    allowed_collection: Addr::unchecked(collection_contract.clone()),
                    lockup_term: vec![
                        LockupTerm {
                            value: 10,
                            percent: Uint128::new(30u128),
                        },
                        LockupTerm {
                            value: 30,
                            percent: Uint128::new(70u128),
                        },
                    ],
                    total_nft_staked: 0,
                    total_reward_claimed: Uint128::from(47500u128),
                    total_reward: Uint128::from(1000000u128),
                    reward_per_second: Uint128::from(10000u128),
                    time_calc_nft: current_block_time + 110,
                    start_time: current_block_time + 10,
                    end_time: current_block_time + 110,
                }
            );
        }

        //         -------------- wrong operation ------------------
        // create campaign with token is native token
        // create campaign with campaign_name.length > 100
        // create campaign with campaign_image, campaign_description > 500
        // create campaign with start_time > end_time
        // create campaign with start_time -> end_time > 3 years
        // add reward with sender is not owner
        // add reward with campaign is active
        // stake nft with reward_per_second = 0
        // stake nft with sender is not owner nft
        // claim reward with sender is not staker
        // claim reward > reward debt staker
        // unstake nft with nft is staking
        // unstake nft with nft is unstaked
        // widthraw reward with campaign is active
        // widthraw reward with sender is not owner campaign
        #[test]
        fn wrong_operation() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();

            // get factory contract
            let factory_contract = &contracts[0].contract_addr;
            // get lp token contract
            let token_contract = &contracts[1].contract_addr;
            // get collection contract
            let collection_contract = &contracts[2].contract_addr;

            // Mint 1000 tokens to ADMIN
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: ADMIN.to_string(),
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // Mint 1000 tokens to USER_1
            let mint_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::Mint {
                recipient: USER_1.to_string(),
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute minting
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &mint_msg,
                &[],
            );

            assert!(response.is_ok());

            // mint 5 nft with token_id = 1..5 to USER_1
            for id in 1..5 {
                // mint nft
                let mint_nft_msg = Cw721MintMsg {
                    token_id: id.to_string(),
                    owner: USER_1.to_string(),
                    token_uri: Some(
                        "https://starships.example.com/Starship/Enterprise.json".into(),
                    ),
                    extension: Some(Metadata {
                        description: Some("Spaceship with Warp Drive".into()),
                        name: Some("Starship USS Enterprise".to_string()),
                        ..Metadata::default()
                    }),
                };

                let exec_msg = Cw721ExecuteMsg::Mint(mint_nft_msg.clone());

                let response_mint_nft = app.execute_contract(
                    Addr::unchecked(ADMIN.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &exec_msg,
                    &[],
                );

                assert!(response_mint_nft.is_ok());
            }

            // mint 5 nft with token_id = 6..10 to USER_2
            for id in 6..10 {
                // mint nft
                let mint_nft_msg = Cw721MintMsg {
                    token_id: id.to_string(),
                    owner: USER_2.to_string(),
                    token_uri: Some(
                        "https://starships.example.com/Starship/Enterprise.json".into(),
                    ),
                    extension: Some(Metadata {
                        description: Some("Spaceship with Warp Drive".into()),
                        name: Some("Starship USS Enterprise".to_string()),
                        ..Metadata::default()
                    }),
                };

                let exec_msg = Cw721ExecuteMsg::Mint(mint_nft_msg.clone());

                let response_mint_nft = app.execute_contract(
                    Addr::unchecked(ADMIN.to_string()),
                    Addr::unchecked(collection_contract.clone()),
                    &exec_msg,
                    &[],
                );

                assert!(response_mint_nft.is_ok());
            }

            // Approve nft to campaign contract
            let approve_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::ApproveAll {
                operator: "contract3".to_string(),
                expires: None,
            };

            // Execute approve nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked(collection_contract.clone()),
                &approve_msg,
                &[],
            );
            assert!(response.is_ok());

            // Approve nft to campaign contract
            let approve_msg: Cw721ExecuteMsg = Cw721ExecuteMsg::ApproveAll {
                operator: "contract3".to_string(),
                expires: None,
            };

            // Execute approve nft
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked(collection_contract.clone()),
                &approve_msg,
                &[],
            );
            assert!(response.is_ok());

            // query balance of ADMIN in cw20 base token contract
            let balance: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    token_contract.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: ADMIN.to_string(),
                    },
                )
                .unwrap();
            // It should be 1000 lp token as minting happened
            assert_eq!(balance.balance, Uint128::from(MOCK_1000_TOKEN_AMOUNT));

            // token info
            let token_info = TokenInfo::Token {
                contract_addr: token_contract.to_string(),
            };

            // get current block time
            let current_block_time = app.block_info().time.seconds();

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 100,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: TokenInfo::NativeToken {
                        denom: "AURA".to_string(),
                    },
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            // wrong with token is native token
            assert!(response_create_campaign.is_err());

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "n".repeat(101).to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 110,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            // wrong with campaign_name.length > 100
            assert!(response_create_campaign.is_err());

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "c".repeat(501).to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 110,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            // wrong with campaign_image.length > 500
            assert!(response_create_campaign.is_err());

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "c".repeat(501).to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 110,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            // wrong with campaign_description.length > 500
            assert!(response_create_campaign.is_err());

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 100,
                end_time: current_block_time + 10,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            // wrong with start_time > end_time
            assert!(response_create_campaign.is_err());

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 94608020,
                limit_per_staker: 2,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            // wrong with end_time - start_time > 3 years
            assert!(response_create_campaign.is_err());

            // create campaign contract by factory contract
            let create_campaign_msg = crate::msg::ExecuteMsg::CreateCampaign {
                owner: ADMIN.to_string(),
                campaign_name: "campaign name".to_string(),
                campaign_image: "campaign name".to_string(),
                campaign_description: "campaign name".to_string(),
                start_time: current_block_time + 10,
                end_time: current_block_time + 110,
                limit_per_staker: 5,
                reward_token_info: AssetToken {
                    info: token_info.clone(),
                    amount: Uint128::zero(),
                },
                allowed_collection: collection_contract.clone(),
                lockup_term: vec![
                    LockupTerm {
                        value: 10,
                        percent: Uint128::new(30u128),
                    },
                    LockupTerm {
                        value: 30,
                        percent: Uint128::new(70u128),
                    },
                ],
            };

            // Execute create campaign
            let response_create_campaign = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(factory_contract.clone()),
                &create_campaign_msg,
                &[],
            );
            assert!(response_create_campaign.is_ok());

            // Approve cw20 token to campaign contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Campaign Contract
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked(token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // Approve cw20 token to campaign contract
            let approve_msg: Cw20ExecuteMsg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: "contract3".to_string(), // Campaign Contract
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
                expires: None,
            };

            // Execute approve
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked(token_contract.clone()),
                &approve_msg,
                &[],
            );

            assert!(response.is_ok());

            // add reward token with USER_1
            let add_reward_balance_msg = CampaignExecuteMsg::AddRewardToken {
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[],
            );

            // err with USER_1 is not owner
            assert!(response.is_err());

            // increase 20 second to make active campaign
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(20),
                height: app.block_info().height + 20,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 1
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "1".to_string(),
                    lockup_term: 10,
                }],
            };
            let _start_time_1 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            // err with reward in campaign = 0
            assert!(response.is_err());

            // add reward token
            let add_reward_balance_msg = CampaignExecuteMsg::AddRewardToken {
                amount: Uint128::from(MOCK_1000_TOKEN_AMOUNT),
            };

            // Execute add reward balance
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &add_reward_balance_msg,
                &[],
            );

            // err with campaign is active
            assert!(response.is_ok());

            // USER_1 claim reward msg
            let claim_reward_msg = CampaignExecuteMsg::ClaimReward {
                amount: Uint128::from(20000u128),
            };

            // Execute claim reward
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &claim_reward_msg,
                &[],
            );

            // err with sender is not staker
            assert!(response.is_err());

            // stake nft token_id 1
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![NftStake {
                    token_id: "1".to_string(),
                    lockup_term: 10,
                }],
            };
            let _start_time_1 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_2.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            // err with USER_2 is not owner token_id 1
            assert!(response.is_err());

            // stake nft token_id 1
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![
                    NftStake {
                        token_id: "1".to_string(),
                        lockup_term: 10,
                    },
                    NftStake {
                        token_id: "2".to_string(),
                        lockup_term: 10,
                    },
                ],
            };
            let start_time_1 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // increase 5 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // stake nft token_id 3,4
            let stake_nft_msg = CampaignExecuteMsg::StakeNfts {
                nfts: vec![
                    NftStake {
                        token_id: "3".to_string(),
                        lockup_term: 10,
                    },
                    NftStake {
                        token_id: "4".to_string(),
                        lockup_term: 10,
                    },
                    // NftStake {
                    //     token_id: "4".to_string(),
                    //     lockup_term: 10,
                    // },
                ],
            };
            let start_time_3 = app.block_info().time.seconds();

            // Execute stake nft to campaign
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &stake_nft_msg,
                &[],
            );

            assert!(response.is_ok());

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(7500u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(7500u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 10
                        },
                        NftInfo {
                            token_id: "4".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(0u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 10
                        }
                    ],
                    reward_debt: Uint128::from(15000u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // USER_1 claim reward msg
            let claim_reward_msg = CampaignExecuteMsg::ClaimReward {
                amount: Uint128::from(20000u128),
            };

            // Execute claim reward
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &claim_reward_msg,
                &[],
            );

            // err with current reward = 15_000 but claim 20_000
            assert!(response.is_err());

            // increase 5 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(5),
                height: app.block_info().height + 5,
                chain_id: app.block_info().chain_id,
            });

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "1".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(11250u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(11250u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(3750u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 10
                        },
                        NftInfo {
                            token_id: "4".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(3750u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 10
                        }
                    ],
                    reward_debt: Uint128::from(30000u128),
                    reward_claimed: Uint128::zero()
                },
            );

            // USER_1 un stake nft msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "3".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            // err with un stake nft is staking
            assert!(response.is_err());

            // USER_1 un stake nft msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "1".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            // err with un stake nft is staking
            assert!(response.is_ok());

            // nft staked with USER_2
            let staked: StakedInfoResult = app
                .wrap()
                .query_wasm_smart(
                    "contract3",
                    &CampaignQueryMsg::NftStaked {
                        owner: Addr::unchecked(USER_1.to_string()),
                    },
                )
                .unwrap();

            assert_eq!(
                staked,
                StakedInfoResult {
                    nfts: vec![
                        NftInfo {
                            token_id: "2".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(11250u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: true,
                            start_time: start_time_1,
                            end_time: start_time_1 + 10
                        },
                        NftInfo {
                            token_id: "3".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(3750u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 10
                        },
                        NftInfo {
                            token_id: "4".to_string(),
                            owner: Addr::unchecked(USER_1.to_string()),
                            pending_reward: Uint128::from(3750u128),
                            lockup_term: LockupTerm {
                                value: 10,
                                percent: Uint128::from(30u128)
                            },
                            is_end_reward: false,
                            start_time: start_time_3,
                            end_time: start_time_3 + 10
                        }
                    ],
                    reward_debt: Uint128::from(30000u128),
                    reward_claimed: Uint128::zero()
                },
            );

            let nfts: Vec<NftInfo> = app
                .wrap()
                .query_wasm_smart("contract3", &CampaignQueryMsg::Nfts { limit: None })
                .unwrap();

            assert_eq!(
                nfts,
                vec![
                    NftInfo {
                        token_id: "2".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(11250u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128)
                        },
                        is_end_reward: true,
                        start_time: start_time_1,
                        end_time: start_time_1 + 10
                    },
                    NftInfo {
                        token_id: "3".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(3750u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128)
                        },
                        is_end_reward: false,
                        start_time: start_time_3,
                        end_time: start_time_3 + 10
                    },
                    NftInfo {
                        token_id: "4".to_string(),
                        owner: Addr::unchecked(USER_1.to_string()),
                        pending_reward: Uint128::from(3750u128),
                        lockup_term: LockupTerm {
                            value: 10,
                            percent: Uint128::from(30u128)
                        },
                        is_end_reward: false,
                        start_time: start_time_3,
                        end_time: start_time_3 + 10
                    }
                ]
            );

            // USER_1 un stake nft msg
            let un_stake_nft_msg = CampaignExecuteMsg::UnStakeNft {
                token_id: "1".to_string(),
            };

            // Execute un stake nft
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &un_stake_nft_msg,
                &[],
            );

            // err with un stake nft is un_staked
            assert!(response.is_err());

            // withdraw remaining reward
            let withdraw_reward_msg = CampaignExecuteMsg::WithdrawReward {};

            // Execute withdraw remaining reward
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_reward_msg,
                &[],
            );

            // err with campaign is active
            assert!(response.is_err());

            // increase 100 second
            app.set_block(BlockInfo {
                time: app.block_info().time.plus_seconds(100),
                height: app.block_info().height + 100,
                chain_id: app.block_info().chain_id,
            });

            // withdraw remaining reward
            let withdraw_reward_msg = CampaignExecuteMsg::WithdrawReward {};

            // Execute withdraw remaining reward
            let response = app.execute_contract(
                Addr::unchecked(USER_1.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_reward_msg,
                &[],
            );

            // err with sender is not owner campaign
            assert!(response.is_err());

            // withdraw remaining reward
            let withdraw_reward_msg = CampaignExecuteMsg::WithdrawReward {};

            // Execute withdraw remaining reward
            let response = app.execute_contract(
                Addr::unchecked(ADMIN.to_string()),
                Addr::unchecked("contract3"),
                &withdraw_reward_msg,
                &[],
            );

            assert!(response.is_ok());
        }

        // -------------- utils test function ------------------
        // calc reward in time
        // overflow when add reward
        // overflow when sub reward
        #[test]
        fn utils_test_function() {
            let start_time: u64 = 10;
            let end_time: u64 = 20;
            let reward_per_second: Uint128 = Uint128::from(10u128);
            let percent: Uint128 = Uint128::from(70u128);
            let nft_count: u128 = 1;

            // check response calc_reward_in_time
            let response =
                calc_reward_in_time(start_time, end_time, reward_per_second, percent, nft_count);
            assert!(response.is_ok());

            let calc_reward = response.unwrap();
            assert_eq!(calc_reward, Uint128::from(70u128));

            // check response calc_reward_in_time error
            let response = calc_reward_in_time(start_time, end_time, reward_per_second, percent, 0);
            assert!(response.is_err());

            // add_reward
            let response = add_reward(Uint128::zero(), calc_reward);
            assert!(response.is_ok());

            let add = response.unwrap();
            assert_eq!(add, Uint128::from(70u128));

            let response = add_reward(Uint128::from(u128::MAX), Uint128::from(10u128));
            assert!(response.is_err());
            // sub_reward
            let response = sub_reward(Uint128::from(20u128), Uint128::from(10u128));
            assert!(response.is_ok());

            let sub = response.unwrap();
            assert_eq!(sub, Uint128::from(10u128));

            let response = sub_reward(Uint128::zero(), calc_reward);

            assert!(response.is_err());
        }
    }
}
