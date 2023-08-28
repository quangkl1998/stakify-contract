#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    has_coins, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    AssetToken, CampaignInfo, CampaignInfoResult, CampaignInfoUpdate, NftInfo, NftStake,
    StakedInfoResult, StakerRewardAssetInfo, TokenInfo, CAMPAIGN_INFO, NFTS, STAKERS_INFO,
    TOKEN_IDS,
};
use crate::utils::{add_reward, calc_reward_in_time, sub_reward};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:campaign";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_TIME_VALID: u64 = 94608000; // 3 years
const MAX_LENGTH_NAME: usize = 100;
const MAX_LENGTH_IMAGE: usize = 500;
const MAX_LENGTH_DESCRIPTION: usize = 500;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set version to contract
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate token contract address
    match msg.reward_token_info.info.clone() {
        TokenInfo::Token { contract_addr } => {
            deps.api.addr_validate(&contract_addr)?;
        }
        TokenInfo::NativeToken { denom: _ } => {
            return Err(ContractError::InvalidToken {});
        }
    }

    // Not allow start time is greater than end time
    if msg.start_time >= msg.end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "## Start time is greater than end time ##",
        )));
    }

    // campaign during max 3 years
    if (msg.end_time - msg.start_time) > MAX_TIME_VALID {
        return Err(ContractError::LimitStartDate {});
    }

    // validate limit character campaign name & campaign description
    if msg.campaign_name.len() > MAX_LENGTH_NAME {
        return Err(ContractError::LimitCharacter {
            max: MAX_LENGTH_NAME.to_string(),
        });
    }

    if msg.campaign_image.len() > MAX_LENGTH_IMAGE {
        return Err(ContractError::LimitCharacter {
            max: MAX_LENGTH_IMAGE.to_string(),
        });
    }

    if msg.campaign_description.len() > MAX_LENGTH_DESCRIPTION {
        return Err(ContractError::LimitCharacter {
            max: MAX_LENGTH_DESCRIPTION.to_string(),
        });
    }

    // // TODO: lockup_term must be 15days, 30days, 60days
    // let lockup_term = &msg.lockup_term;
    // for term in lockup_term {
    //     if Term::from_value(&term.value).is_none() {
    //         return Err(ContractError::InvalidLockupTerm {});
    //     }
    // }

    // campaign info
    let campaign = CampaignInfo {
        owner: deps.api.addr_validate(&msg.owner).unwrap(),
        campaign_name: msg.campaign_name.clone(),
        campaign_image: msg.campaign_image.clone(),
        campaign_description: msg.campaign_description.clone(),
        total_reward_claimed: Uint128::zero(),
        total_reward: Uint128::zero(),
        limit_per_staker: msg.limit_per_staker,
        reward_token: AssetToken {
            info: msg.reward_token_info.info.clone(),
            amount: Uint128::zero(),
        },
        allowed_collection: deps.api.addr_validate(&msg.allowed_collection).unwrap(),
        lockup_term: msg.lockup_term.clone(),
        reward_per_second: Uint128::zero(),
        time_calc_nft: 0,
        start_time: msg.start_time,
        end_time: msg.end_time,
    };

    // save campaign info
    CAMPAIGN_INFO.save(deps.storage, &campaign)?;

    // init TOKEN_IDS to vec![]
    TOKEN_IDS.save(deps.storage, &vec![])?;

    // we need emit the information of reward token to response
    let reward_token_info_str = match msg.reward_token_info.info {
        TokenInfo::Token { contract_addr } => contract_addr,
        TokenInfo::NativeToken { denom } => denom,
    };

    // emit the information of instantiated campaign
    Ok(Response::new().add_attributes([
        ("action", "instantiate"),
        ("owner", &msg.owner),
        ("campaign_name", &msg.campaign_name),
        ("campaign_image", &msg.campaign_image),
        ("campaign_description", &msg.campaign_description),
        ("limit_per_staker", &msg.limit_per_staker.to_string()),
        ("reward_token_info", &reward_token_info_str),
        ("allowed_collection", &msg.allowed_collection),
        ("lockup_term", &format!("{:?}", &msg.lockup_term)),
        ("start_time", &msg.start_time.to_string()),
        ("end_time", &msg.end_time.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddRewardToken { amount } => execute_add_reward_token(deps, env, info, amount),
        ExecuteMsg::StakeNfts { nfts } => execute_stake_nft(deps, env, info, nfts),
        ExecuteMsg::UnStakeNft { token_id } => execute_unstake_nft(deps, env, info, token_id),
        ExecuteMsg::ClaimReward { amount } => execute_claim_reward(deps, env, info, amount),
        ExecuteMsg::WithdrawReward {} => execute_withdraw_reward(deps, env, info),
        ExecuteMsg::UpdateCampaign {
            campaign_info_update,
        } => execute_update_campaign(deps, env, info, campaign_info_update),
    }
}

pub fn execute_add_reward_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // load campaign info
    let mut campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    let current_time = env.block.time.seconds();

    // only owner can add reward token to campaign
    if campaign_info.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // // cannot add reward twice
    // if campaign_info.reward_token.amount != Uint128::zero() {
    //     return Err(ContractError::RewardAdded {});
    // }

    // // cannot add reward if campaign is started
    // if campaign_info.start_time <= current_time {
    //     return Err(ContractError::InvalidTimeToAddReward {});
    // }

    // only reward_per_second == 0 || start_time > current_time can add reward
    if campaign_info.reward_per_second != Uint128::zero()
        && campaign_info.start_time <= current_time
    {
        return Err(ContractError::InvalidTimeToAddReward {});
    }

    let mut res = Response::new();

    // we need determine the reward token is native token or cw20 token
    match campaign_info.reward_token.info.clone() {
        TokenInfo::Token { contract_addr } => {
            // execute cw20 transfer msg from info.sender to contract
            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount,
                })?,
                funds: vec![],
            }));

            // add token info to response
            res = res.add_attribute("reward_token_info", contract_addr);

            // update amount, reward_per_second token in campaign
            campaign_info.reward_token.amount = campaign_info
                .reward_token
                .amount
                .checked_add(amount)
                .unwrap();
            campaign_info.reward_per_second = campaign_info
                .reward_token
                .amount
                .checked_div(Uint128::from(
                    campaign_info.end_time - campaign_info.start_time,
                ))
                .unwrap();
            campaign_info.total_reward = campaign_info.total_reward.checked_add(amount).unwrap();

            // save campaign
            CAMPAIGN_INFO.save(deps.storage, &campaign_info)?;
        }
        TokenInfo::NativeToken { denom } => {
            // check the amount of native token in funds
            if !has_coins(
                &info.funds,
                &Coin {
                    denom: denom.clone(),
                    amount,
                },
            ) {
                return Err(ContractError::InvalidFunds {});
            }

            // add token info to response
            res = res.add_attribute("reward_token_info", &denom);
        }
    }
    Ok(res.add_attributes([
        ("action", "add_reward_token"),
        ("owner", campaign_info.owner.as_ref()),
        ("reward_token_amount", &amount.to_string()),
    ]))
}

pub fn execute_stake_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    nfts: Vec<NftStake>,
) -> Result<Response, ContractError> {
    // load campaign info
    let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    let current_time = env.block.time.seconds();

    // the reward token must be added to campaign before staking nft
    if campaign_info.reward_token.amount == Uint128::zero() {
        return Err(ContractError::EmptyReward {});
    }

    // only start_time < current_time && current_time < end_time && amount != 0 can stake nft
    if campaign_info.start_time >= current_time || campaign_info.end_time <= current_time {
        return Err(ContractError::InvalidTimeToStakeNft {});
    }

    // load staker_info or default if staker has not staked nft
    let mut staker_info = STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(StakerRewardAssetInfo {
            token_ids: vec![],
            reward_debt: Uint128::zero(),
            reward_claimed: Uint128::zero(),
        });

    // if limit per staker > 0 then check amount nft staked
    // if limit_per_staker = 0, then no limit nft stake
    if campaign_info.limit_per_staker > 0 {
        // the length of token_ids + length nft staked should be smaller than limit per staker
        if nfts.len() + staker_info.token_ids.len() > campaign_info.limit_per_staker as usize {
            return Err(ContractError::LimitPerStake {});
        }
    }

    // prepare response
    let mut res = Response::new();

    // list token_ids
    let mut token_ids = TOKEN_IDS.load(deps.storage)?;

    // if nft is first stake then skip calc reward
    if campaign_info.time_calc_nft != 0 {
        // update pending reward for previous staking nft
        let terms = campaign_info.clone().lockup_term;
        let time_calc_nft = campaign_info.time_calc_nft;
        let reward_per_second = campaign_info.reward_per_second;

        // load nfts
        let mut nfts_load = Vec::new();
        let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
        for item in nfts_storage {
            let (_, nft_info) = item?;

            // skip nft is end reward
            if !nft_info.is_end_reward {
                nfts_load.push(nft_info);
            }
        }

        // filter nft by term
        for term in terms {
            let mut nft_list = nfts_load
                .clone()
                .into_iter()
                .filter(|nft| nft.lockup_term.value == term.value)
                .collect::<Vec<_>>();
            nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

            let mut time_calc: u64 = time_calc_nft;
            let mut nft_count = nft_list.len() as u128;
            let mut reward = Uint128::zero();
            for nft in nft_list.iter_mut() {
                if nft.end_time <= current_time {
                    // calc in time_calc -> nft.end_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        nft.end_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // increase reward for next nft
                    reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                    nft_count -= 1; // update count nft for next calc reward
                    time_calc = nft.end_time; // update time_calc
                    nft.is_end_reward = true; // nft stake timeout
                } else {
                    // calc in time_calc -> current_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        current_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // add reward previous and current reward
                    let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap();
                }
                // save nfts
                NFTS.save(deps.storage, nft.token_id.clone(), nft)?;
            }
        }
    }

    // check the owner of token_ids, all token_ids should be owned by info.sender
    for nft in &nfts {
        // let campaign_info = CAMPAIGN_INFO.load(deps.storage)?;
        // check invalid lockup_term
        if !campaign_info
            .clone()
            .lockup_term
            .iter()
            .any(|t| t.value == nft.lockup_term)
        {
            return Err(ContractError::InvalidLockupTerm {});
        }

        // check owner of nft
        let query_owner_msg = Cw721QueryMsg::OwnerOf {
            token_id: nft.token_id.clone(),
            include_expired: Some(false),
        };

        let owner_response: StdResult<cw721::OwnerOfResponse> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: campaign_info.allowed_collection.clone().to_string(),
                msg: to_binary(&query_owner_msg)?,
            }));
        match owner_response {
            Ok(owner) => {
                if owner.owner != info.sender {
                    return Err(ContractError::NotOwner {
                        token_id: nft.token_id.to_string(),
                    });
                }
            }
            Err(_) => {
                return Err(ContractError::NotOwner {
                    token_id: nft.token_id.to_string(),
                });
            }
        }

        // prepare message to transfer nft to contract
        let transfer_nft_msg = WasmMsg::Execute {
            contract_addr: campaign_info.allowed_collection.clone().to_string(),
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: env.contract.address.clone().to_string(),
                token_id: nft.token_id.clone(),
            })?,
            funds: vec![],
        };

        // load lockup_term in campaign info
        let lockup_term = campaign_info
            .lockup_term
            .iter()
            .find(|&term| term.value == nft.lockup_term)
            .cloned()
            .unwrap();

        let nft_info = NftInfo {
            token_id: nft.token_id.clone(),
            owner: info.sender.clone(),
            pending_reward: Uint128::zero(),
            lockup_term: lockup_term.clone(),
            is_end_reward: false,
            start_time: current_time,
            end_time: (current_time + lockup_term.value),
        };
        // save info nft
        NFTS.save(deps.storage, nft.token_id.clone(), &nft_info)?;

        // save staker_info
        staker_info.token_ids.push(nft.token_id.clone());

        token_ids.push(nft.token_id.clone());

        res = res.add_message(transfer_nft_msg);
    }

    STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;

    // update time calc pending reward for nft
    let mut update_campaign = campaign_info.clone();
    update_campaign.time_calc_nft = current_time;
    CAMPAIGN_INFO.save(deps.storage, &update_campaign)?;

    // save TOKEN_IDS
    TOKEN_IDS.save(deps.storage, &token_ids)?;

    Ok(res.add_attributes([
        ("action", "stake_nft"),
        ("owner", info.sender.as_ref()),
        (
            "allowed_collection",
            campaign_info.allowed_collection.as_ref(),
        ),
        ("nfts", &format!("{:?}", &nfts)),
    ]))
}

pub fn execute_unstake_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // load campaign info
    let mut campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;
    // prepare response
    let mut res = Response::new();

    if NFTS.may_load(deps.storage, token_id.clone())?.is_none() {
        return Err(ContractError::EmptyNft { token_id });
    }

    // max time calc pending reward is campaign_info.end_time
    let mut current_time = env.block.time.seconds();
    if campaign_info.end_time < env.block.time.seconds() {
        current_time = campaign_info.end_time;
    }

    // update pending reward for previous staking nft
    let terms = campaign_info.clone().lockup_term;
    let time_calc_nft = campaign_info.time_calc_nft;
    let reward_per_second = campaign_info.reward_per_second;

    // load nfts
    let mut nfts_load = Vec::new();
    let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
    for item in nfts_storage {
        let (_, nft_info) = item?;
        if !nft_info.is_end_reward {
            nfts_load.push(nft_info);
        }
    }

    for term in terms {
        let mut nft_list = nfts_load
            .clone()
            .into_iter()
            .filter(|nft| nft.lockup_term.value == term.value)
            .collect::<Vec<_>>();
        nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let mut time_calc: u64 = time_calc_nft;
        let mut nft_count = nft_list.len() as u128;
        let mut reward = Uint128::zero();
        for nft in nft_list.iter_mut() {
            if nft.end_time <= current_time {
                // calc in time_calc -> nft.end_time
                let calc_reward = calc_reward_in_time(
                    time_calc,
                    nft.end_time,
                    reward_per_second,
                    term.percent,
                    nft_count,
                )
                .unwrap();

                // increase reward for next nft
                reward = add_reward(reward, calc_reward).unwrap();

                // update reward for nft
                nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                nft_count -= 1; // update count nft for next calc reward
                time_calc = nft.end_time; // update time_calc
                nft.is_end_reward = true; // nft stake timeout
            } else {
                // calc in time_calc -> current_time
                let calc_reward = calc_reward_in_time(
                    time_calc,
                    current_time,
                    reward_per_second,
                    term.percent,
                    nft_count,
                )
                .unwrap();

                // add reward previous and current reward
                let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                // update reward for nft
                nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap();
            }
            if env.block.time.seconds() >= campaign_info.end_time {
                nft.is_end_reward = true;
            }
            // save nfts
            NFTS.save(deps.storage, nft.token_id.clone(), nft)?;
        }
    }

    // update time calc pending reward for nft
    campaign_info.time_calc_nft = current_time;
    CAMPAIGN_INFO.save(deps.storage, &campaign_info)?;

    // load nft info
    let nft_info = NFTS.load(deps.storage, token_id.clone())?;

    // check time unstake and owner nft
    if !nft_info.is_end_reward {
        return Err(ContractError::InvalidTimeToUnStake {});
    }

    // prepare message to transfer nft back to the owner
    let transfer_nft_msg = WasmMsg::Execute {
        contract_addr: campaign_info.allowed_collection.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id: token_id.clone(),
        })?,
        funds: vec![],
    };

    // remove nft in NFTS
    NFTS.remove(deps.storage, token_id.clone());

    // remove token_id in TOKEN_IDS
    let mut token_ids = TOKEN_IDS.load(deps.storage)?;
    token_ids.retain(|id| *id != token_id.clone());
    TOKEN_IDS.save(deps.storage, &token_ids)?;

    // update reward for staker
    let mut staker = STAKERS_INFO.load(deps.storage, info.sender.clone())?;
    staker.reward_debt = add_reward(staker.reward_debt, nft_info.pending_reward).unwrap();
    staker.token_ids.retain(|key| *key != token_id.clone()); // remove nft for staker
    STAKERS_INFO.save(deps.storage, info.sender.clone(), &staker)?;

    res = res.add_message(transfer_nft_msg);
    Ok(res.add_attributes([
        ("action", "unstake_nft"),
        ("owner", info.sender.as_ref()),
        (
            "allowed_collection",
            campaign_info.allowed_collection.as_ref(),
        ),
        ("token_id", &token_id),
    ]))
}

pub fn execute_claim_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // load campaign info
    let mut campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    // Only stakers could claim rewards in this campaign
    if STAKERS_INFO
        .may_load(deps.storage, info.sender.clone())?
        .is_none()
    {
        return Err(ContractError::InvalidClaim {});
    }

    // load staker_info
    let mut staker_info = STAKERS_INFO.load(deps.storage, info.sender.clone())?;

    // max time calc pending reward is campaign_info.end_time
    let mut current_time = env.block.time.seconds();
    if campaign_info.end_time < env.block.time.seconds() {
        current_time = campaign_info.end_time;
    }

    // update pending reward for previous staking nft
    let terms = campaign_info.clone().lockup_term;
    let time_calc_nft = campaign_info.time_calc_nft;
    let reward_per_second = campaign_info.reward_per_second;

    // load nfts
    let mut nfts_load = Vec::new();
    let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
    for item in nfts_storage {
        let (_, nft_info) = item?;
        if !nft_info.is_end_reward {
            nfts_load.push(nft_info);
        }
    }

    for term in terms {
        let mut nft_list = nfts_load
            .clone()
            .into_iter()
            .filter(|nft| nft.lockup_term.value == term.value)
            .collect::<Vec<_>>();
        nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let mut time_calc: u64 = time_calc_nft;
        let mut nft_count = nft_list.len() as u128;
        let mut reward = Uint128::zero();
        for nft in nft_list.iter_mut() {
            if nft.end_time <= current_time {
                // calc in time_calc -> nft.end_time
                let calc_reward = calc_reward_in_time(
                    time_calc,
                    nft.end_time,
                    reward_per_second,
                    term.percent,
                    nft_count,
                )
                .unwrap();

                // increase reward for next nft
                reward = add_reward(reward, calc_reward).unwrap();

                // update reward for nft
                nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                nft_count -= 1; // update count nft for next calc reward
                time_calc = nft.end_time; // update time_calc
                nft.is_end_reward = true; // nft stake timeout
            } else {
                // calc in time_calc -> current_time
                let calc_reward = calc_reward_in_time(
                    time_calc,
                    current_time,
                    reward_per_second,
                    term.percent,
                    nft_count,
                )
                .unwrap();

                // add reward previous and current reward
                let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                // update reward for nft
                nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap();
            }

            // if campaign is timeout -> nft timeout
            if env.block.time.seconds() >= campaign_info.end_time {
                nft.is_end_reward = true;
            }
            // save nfts
            NFTS.save(deps.storage, nft.token_id.clone(), nft)?;
        }
    }

    // update time calc pending reward for nft
    campaign_info.time_calc_nft = current_time;

    // transfer pending reward in nft to staker
    for id in staker_info.token_ids.iter() {
        let mut nft = NFTS.load(deps.storage, id.clone())?;
        staker_info.reward_debt = add_reward(staker_info.reward_debt, nft.pending_reward).unwrap();

        //update pending reward for nft = 0 because pending reward in nft are transferred to staker
        nft.pending_reward = Uint128::zero();
        NFTS.save(deps.storage, id.clone(), &nft)?;
    }

    // amount reward claim must be less than or equal reward in staker
    if amount > staker_info.reward_debt {
        return Err(ContractError::InsufficientBalance {});
    }

    match campaign_info.reward_token.info.clone() {
        TokenInfo::Token { contract_addr } => {
            // check balance
            let query_balance_msg = Cw20QueryMsg::Balance {
                address: env.contract.address.to_string(),
            };
            let balance_response: StdResult<cw20::BalanceResponse> =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&query_balance_msg)?,
                }));
            match balance_response {
                Ok(balance) => {
                    if balance.balance < amount {
                        return Err(ContractError::InsufficientBalance {});
                    }
                }
                Err(_) => {
                    return Err(ContractError::InsufficientBalance {});
                }
            }

            // execute cw20 transfer msg from info.sender to contract
            let transfer_reward: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount,
                })?,
                funds: vec![],
            });

            // update staker info
            staker_info.reward_claimed = add_reward(staker_info.reward_claimed, amount).unwrap();
            staker_info.reward_debt = sub_reward(staker_info.reward_debt, amount).unwrap();
            STAKERS_INFO.save(deps.storage, info.sender, &staker_info)?;

            // update reward total and reward claimed for campaign
            campaign_info.reward_token.amount =
                sub_reward(campaign_info.reward_token.amount, amount).unwrap();
            campaign_info.total_reward_claimed =
                add_reward(campaign_info.total_reward_claimed, amount).unwrap();

            // save campaign info
            CAMPAIGN_INFO.save(deps.storage, &campaign_info)?;

            Ok(Response::new()
                .add_message(transfer_reward)
                .add_attributes([
                    ("action", "claim_reward"),
                    ("owner", campaign_info.owner.as_ref()),
                    ("reward_token_info", contract_addr.as_ref()),
                    ("reward_claim_amount", &amount.to_string()),
                ]))
        }
        TokenInfo::NativeToken { denom } => {
            // check the amount of native token in funds
            if !has_coins(
                &info.funds,
                &Coin {
                    denom: denom.clone(),
                    amount,
                },
            ) {
                return Err(ContractError::InvalidFunds {});
            }

            Ok(Response::new().add_attributes([
                ("action", "claim_reward"),
                ("owner", campaign_info.owner.as_ref()),
                ("denom", &denom),
                ("reward_claim_amount", &amount.to_string()),
            ]))
        }
    }
}

pub fn execute_withdraw_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // load campaign info
    let mut campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    // permission check
    if info.sender != campaign_info.owner {
        return Err(ContractError::Unauthorized {});
    }

    // campaing must be ended then withdraw remaining reward
    if campaign_info.end_time > env.block.time.seconds() {
        return Err(ContractError::InvalidTimeToWithdrawReward {});
    }

    // total_pending_reward = total reward in nfts + total reward in stakers
    let mut total_pending_reward = Uint128::zero();

    // time to calc pending reward
    let current_time = campaign_info.end_time;

    // update pending reward for all nft
    let terms = campaign_info.clone().lockup_term;
    let time_calc_nft = campaign_info.time_calc_nft;
    let reward_per_second = campaign_info.reward_per_second;

    // load nfts
    let mut nfts_load = Vec::new();
    let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
    for item in nfts_storage {
        let (_, nft_info) = item?;
        if !nft_info.is_end_reward {
            nfts_load.push(nft_info);
        }
    }

    for term in terms {
        let mut nft_list = nfts_load
            .clone()
            .into_iter()
            .filter(|nft| nft.lockup_term.value == term.value)
            .collect::<Vec<_>>();
        nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let mut time_calc: u64 = time_calc_nft;
        let mut nft_count = nft_list
            .clone()
            .into_iter()
            .filter(|nft| !nft.is_end_reward)
            .collect::<Vec<_>>()
            .len() as u128;
        let mut reward = Uint128::zero();
        for nft in nft_list.iter_mut() {
            if !nft.is_end_reward {
                if nft.end_time <= current_time {
                    // calc in time_calc -> nft.end_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        nft.end_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // increase reward for next nft
                    reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                    nft_count -= 1; // update count nft for next calc reward
                    time_calc = nft.end_time; // update time_calc
                    nft.is_end_reward = true; // nft stake timeout
                } else {
                    // calc in time_calc -> current_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        current_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // add reward previous and current reward
                    let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap()
                }
                nft.is_end_reward = true;
            }

            // pending reward in nft
            total_pending_reward = add_reward(total_pending_reward, nft.pending_reward).unwrap();

            // save nfts
            NFTS.save(deps.storage, nft.token_id.clone(), nft)?;
        }
    }

    // pending reward in staker
    let stakers_info = STAKERS_INFO.range(deps.storage, None, None, Order::Ascending);
    for item in stakers_info {
        let (_, value) = item?;
        total_pending_reward = add_reward(total_pending_reward, value.reward_debt).unwrap();
    }

    // update time calc pending reward for nft
    campaign_info.time_calc_nft = current_time;
    CAMPAIGN_INFO.save(deps.storage, &campaign_info)?;

    // reward remaining = reward in campaign - total pending reward
    let withdraw_reward = campaign_info
        .reward_token
        .amount
        .checked_sub(total_pending_reward)
        .unwrap();

    match campaign_info.reward_token.info.clone() {
        TokenInfo::Token { contract_addr } => {
            // check balance
            let query_balance_msg = Cw20QueryMsg::Balance {
                address: env.contract.address.to_string(),
            };
            let balance_response: StdResult<cw20::BalanceResponse> =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&query_balance_msg)?,
                }));
            match balance_response {
                Ok(balance) => {
                    if balance.balance < withdraw_reward {
                        return Err(ContractError::InsufficientBalance {});
                    }
                }
                Err(_) => {
                    return Err(ContractError::InsufficientBalance {});
                }
            }

            // execute cw20 transfer msg from info.sender to contract
            let transfer_reward: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: withdraw_reward,
                })?,
                funds: vec![],
            });

            // update reward total and reward claimed for campaign
            campaign_info.reward_token.amount =
                sub_reward(campaign_info.reward_token.amount, withdraw_reward).unwrap();
            CAMPAIGN_INFO.save(deps.storage, &campaign_info)?;

            Ok(Response::new()
                .add_message(transfer_reward)
                .add_attributes([
                    ("action", "withdraw_reward"),
                    ("owner", campaign_info.owner.as_ref()),
                    ("reward_token_info", contract_addr.as_ref()),
                    ("withdraw_reward_amount", &withdraw_reward.to_string()),
                ]))
        }
        TokenInfo::NativeToken { denom } => {
            // check the amount of native token in funds
            if !has_coins(
                &info.funds,
                &Coin {
                    denom: denom.clone(),
                    amount: withdraw_reward,
                },
            ) {
                return Err(ContractError::InvalidFunds {});
            }

            Ok(Response::new().add_attributes([
                ("action", "claim_reward"),
                ("owner", campaign_info.owner.as_ref()),
                ("denom", &denom),
                ("reward_claim_amount", &withdraw_reward.to_string()),
            ]))
        }
    }
}

pub fn execute_update_campaign(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    campaign_info_update: CampaignInfoUpdate,
) -> Result<Response, ContractError> {
    // load campaign info
    let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    let current_time = env.block.time.seconds();

    // permission check
    if info.sender != campaign_info.owner {
        return Err(ContractError::Unauthorized {});
    }

    // only campaign not yet add reward can update,
    if campaign_info.total_reward != Uint128::zero() {
        return Err(ContractError::InvalidTimeToUpdate {});
    }

    let update_start_time = if let Some(st) = campaign_info_update.start_time {
        st
    } else {
        campaign_info.start_time
    };
    let update_end_time = if let Some(et) = campaign_info_update.end_time {
        et
    } else {
        campaign_info.end_time
    };

    let update_name = if let Some(name) = campaign_info_update.campaign_name {
        name
    } else {
        campaign_info.campaign_name
    };
    let update_image = if let Some(image) = campaign_info_update.campaign_image {
        image
    } else {
        campaign_info.campaign_image
    };
    let update_description = if let Some(description) = campaign_info_update.campaign_description {
        description
    } else {
        campaign_info.campaign_description
    };

    let update_limit_per_staker = if let Some(limit_nft) = campaign_info_update.limit_per_staker {
        limit_nft
    } else {
        campaign_info.limit_per_staker
    };

    let update_lockup_term = if let Some(lockup_term) = campaign_info_update.lockup_term {
        lockup_term
    } else {
        campaign_info.lockup_term
    };

    // campaign during max 3 years
    if (update_end_time - update_start_time) > MAX_TIME_VALID {
        return Err(ContractError::LimitStartDate {});
    }

    // validate character limit campaign name & campaign description
    if update_name.len() > MAX_LENGTH_NAME {
        return Err(ContractError::LimitCharacter {
            max: MAX_LENGTH_NAME.to_string(),
        });
    }
    if update_description.len() > MAX_LENGTH_DESCRIPTION {
        return Err(ContractError::LimitCharacter {
            max: MAX_LENGTH_DESCRIPTION.to_string(),
        });
    }

    // Not allow start time is greater than end time
    if update_start_time >= update_end_time {
        return Err(ContractError::Std(StdError::generic_err(
            "## Start time is greater than end time ##",
        )));
    }

    // Not allow to create a campaign when current time is greater than start time
    if current_time > update_start_time {
        return Err(ContractError::Std(StdError::generic_err(
            "## Current time is greater than start time ##",
        )));
    }

    let campaign_info = CampaignInfo {
        owner: campaign_info.owner.clone(),
        campaign_name: update_name,
        campaign_image: update_image,
        campaign_description: update_description,
        time_calc_nft: campaign_info.time_calc_nft,
        start_time: update_start_time,
        end_time: update_end_time,
        total_reward_claimed: campaign_info.total_reward_claimed,
        total_reward: campaign_info.total_reward,
        limit_per_staker: update_limit_per_staker,
        reward_token: campaign_info.reward_token,
        allowed_collection: campaign_info.allowed_collection,
        lockup_term: update_lockup_term,
        reward_per_second: campaign_info.reward_per_second,
    };

    // save update campaign info
    CAMPAIGN_INFO.save(deps.storage, &campaign_info)?;

    Ok(Response::new().add_attributes([
        ("action", "update_campaign"),
        ("owner", campaign_info.owner.as_ref()),
        ("campaign_name", &campaign_info.campaign_name),
        ("campaign_image", &campaign_info.campaign_image),
        ("campaign_description", &campaign_info.campaign_description),
        (
            "limit_per_staker",
            &campaign_info.limit_per_staker.to_string(),
        ),
        (
            "reward_token_info",
            &format!("{:?}", &campaign_info.reward_token),
        ),
        (
            "allowed_collection",
            campaign_info.allowed_collection.as_ref(),
        ),
        ("lockup_term", &format!("{:?}", &campaign_info.lockup_term)),
        ("start_time", &update_start_time.to_string()),
        ("end_time", &update_end_time.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::CampaignInfo {} => Ok(to_binary(&query_campaign_info(deps)?)?),
        QueryMsg::NftInfo { token_id } => Ok(to_binary(&query_nft_info(deps, env, token_id)?)?),
        QueryMsg::Nft { token_id } => Ok(to_binary(&query_nft(deps, env, token_id)?)?),
        QueryMsg::NftStaked { owner } => Ok(to_binary(&query_staker_info(deps, env, owner)?)?),
        QueryMsg::Nfts { limit } => Ok(to_binary(&query_nfts(deps, env, limit)?)?),
        QueryMsg::TotalPendingReward {} => Ok(to_binary(&query_total_pending_reward(deps, env)?)?),
        QueryMsg::TokenIds {} => Ok(to_binary(&query_token_ids(deps)?)?),
    }
}

fn query_campaign_info(deps: Deps) -> Result<CampaignInfoResult, ContractError> {
    let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    let total_nft_staked = TOKEN_IDS.load(deps.storage)?.len() as u64;

    let campaign_result = CampaignInfoResult {
        owner: campaign_info.owner,
        campaign_name: campaign_info.campaign_name,
        campaign_image: campaign_info.campaign_image,
        campaign_description: campaign_info.campaign_description,
        start_time: campaign_info.start_time,
        end_time: campaign_info.end_time,
        total_reward_claimed: campaign_info.total_reward_claimed,
        total_reward: campaign_info.total_reward,
        limit_per_staker: campaign_info.limit_per_staker,
        reward_token_info: campaign_info.reward_token,
        allowed_collection: campaign_info.allowed_collection,
        lockup_term: campaign_info.lockup_term,
        reward_per_second: campaign_info.reward_per_second,
        time_calc_nft: campaign_info.time_calc_nft,
        total_nft_staked,
    };
    Ok(campaign_result)
}

fn query_nft_info(deps: Deps, env: Env, token_id: String) -> Result<NftInfo, ContractError> {
    let mut info: NftInfo = NFTS.load(deps.storage, token_id)?;

    // if nft is active then calculate
    if !info.is_end_reward {
        // get time to calc pending reward
        let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;
        let mut current_time = env.block.time.seconds();
        if campaign_info.end_time < env.block.time.seconds() {
            current_time = campaign_info.end_time;
        }
        // update pending reward for all nft
        let terms = campaign_info.clone().lockup_term;
        let time_calc_nft = campaign_info.time_calc_nft;
        let reward_per_second = campaign_info.reward_per_second;

        // load nfts
        let mut nfts_load = Vec::new();
        let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
        for item in nfts_storage {
            let (_, nft_info) = item?;
            if !nft_info.is_end_reward {
                nfts_load.push(nft_info);
            }
        }

        for term in terms {
            let mut nft_list = nfts_load
                .clone()
                .into_iter()
                .filter(|nft| nft.lockup_term.value == term.value)
                .collect::<Vec<_>>();
            nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

            let mut time_calc: u64 = time_calc_nft;
            let mut nft_count = nft_list
                .clone()
                .into_iter()
                .filter(|nft| !nft.is_end_reward)
                .collect::<Vec<_>>()
                .len() as u128;
            let mut reward = Uint128::zero();
            for nft in nft_list.iter_mut() {
                if !nft.is_end_reward {
                    if nft.end_time <= current_time {
                        // calc in time_calc -> nft.end_time
                        let calc_reward = calc_reward_in_time(
                            time_calc,
                            nft.end_time,
                            reward_per_second,
                            term.percent,
                            nft_count,
                        )
                        .unwrap();

                        // increase reward for next nft
                        reward = add_reward(reward, calc_reward).unwrap();

                        // update reward for nft
                        nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                        nft_count -= 1; // update count nft for next calc reward
                        time_calc = nft.end_time; // update time_calc
                        nft.is_end_reward = true; // nft stake timeout
                    } else {
                        // calc in time_calc -> current_time
                        let calc_reward = calc_reward_in_time(
                            time_calc,
                            current_time,
                            reward_per_second,
                            term.percent,
                            nft_count,
                        )
                        .unwrap();

                        // add reward previous and current reward
                        let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                        // update reward for nft
                        nft.pending_reward =
                            add_reward(nft.pending_reward, accumulate_reward).unwrap()
                    }
                    if env.block.time.seconds() >= campaign_info.end_time {
                        nft.is_end_reward = true;
                    }
                }
                if info.token_id == nft.token_id {
                    info = nft.clone();
                }
            }
        }
    }

    Ok(info)
}

fn query_nft(deps: Deps, _env: Env, token_id: String) -> Result<NftInfo, ContractError> {
    let info: NftInfo = NFTS.load(deps.storage, token_id)?;

    Ok(info)
}

fn query_staker_info(deps: Deps, env: Env, owner: Addr) -> Result<StakedInfoResult, ContractError> {
    let staker_asset: StakerRewardAssetInfo = STAKERS_INFO
        .may_load(deps.storage, owner)?
        .unwrap_or(StakerRewardAssetInfo {
            token_ids: vec![],
            reward_debt: Uint128::zero(),
            reward_claimed: Uint128::zero(),
        });

    let mut staked_info = StakedInfoResult {
        nfts: vec![],
        reward_debt: staker_asset.reward_debt,
        reward_claimed: staker_asset.reward_claimed,
    };

    // get time to calc pending reward
    let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;
    let mut current_time = env.block.time.seconds();
    if campaign_info.end_time < env.block.time.seconds() {
        current_time = campaign_info.end_time;
    }

    // update pending reward for all nft
    let terms = campaign_info.clone().lockup_term;
    let time_calc_nft = campaign_info.time_calc_nft;
    let reward_per_second = campaign_info.reward_per_second;

    // load nfts
    let mut nfts_load = Vec::new();
    let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
    for item in nfts_storage {
        let (_, nft_info) = item?;
        nfts_load.push(nft_info);
    }

    for term in terms {
        let mut nft_list = nfts_load
            .clone()
            .into_iter()
            .filter(|nft| nft.lockup_term.value == term.value)
            .collect::<Vec<_>>();
        nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let mut time_calc: u64 = time_calc_nft;
        let mut nft_count = nft_list
            .clone()
            .into_iter()
            .filter(|nft| !nft.is_end_reward)
            .collect::<Vec<_>>()
            .len() as u128;
        let mut reward = Uint128::zero();
        for nft in nft_list.iter_mut() {
            if !nft.is_end_reward {
                if nft.end_time <= current_time {
                    // calc in time_calc -> nft.end_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        nft.end_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // increase reward for next nft
                    reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                    nft_count -= 1; // update count nft for next calc reward
                    time_calc = nft.end_time; // update time_calc
                    nft.is_end_reward = true; // nft stake timeout
                } else {
                    // calc in time_calc -> current_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        current_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // add reward previous and current reward
                    let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap()
                }
                if env.block.time.seconds() >= campaign_info.end_time {
                    nft.is_end_reward = true;
                }
            }
            if staker_asset.token_ids.contains(&nft.token_id) {
                staked_info.nfts.push(nft.clone());
                staked_info.reward_debt =
                    add_reward(staked_info.reward_debt, nft.pending_reward).unwrap();
            }
        }
    }

    Ok(staked_info)
}

fn query_nfts(deps: Deps, env: Env, limit: Option<u32>) -> Result<Vec<NftInfo>, ContractError> {
    let mut result_nfts = vec![];

    let limit = limit.unwrap_or(30 as u32) as usize;
    // get time to calc pending reward
    let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;
    let mut current_time = env.block.time.seconds();
    if campaign_info.end_time < env.block.time.seconds() {
        current_time = campaign_info.end_time;
    }
    // update pending reward for all nft
    let terms = campaign_info.clone().lockup_term;
    let time_calc_nft = campaign_info.time_calc_nft;
    let reward_per_second = campaign_info.reward_per_second;

    // load nfts
    let mut nfts_load = Vec::new();
    let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
    for item in nfts_storage {
        let (_, nft_info) = item?;
        nfts_load.push(nft_info);
    }

    for term in terms {
        let mut nft_list = nfts_load
            .clone()
            .into_iter()
            .filter(|nft| nft.lockup_term.value == term.value)
            .collect::<Vec<_>>();
        nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let mut time_calc: u64 = time_calc_nft;
        let mut nft_count = nft_list
            .clone()
            .into_iter()
            .filter(|nft| !nft.is_end_reward)
            .collect::<Vec<_>>()
            .len() as u128;
        let mut reward = Uint128::zero();
        for nft in nft_list.iter_mut() {
            if !nft.is_end_reward {
                if nft.end_time <= current_time {
                    // calc in time_calc -> nft.end_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        nft.end_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // increase reward for next nft
                    reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                    nft_count -= 1; // update count nft for next calc reward
                    time_calc = nft.end_time; // update time_calc
                    nft.is_end_reward = true; // nft stake timeout
                } else {
                    // calc in time_calc -> current_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        current_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // add reward previous and current reward
                    let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap()
                }
                if env.block.time.seconds() >= campaign_info.end_time {
                    nft.is_end_reward = true;
                }
            }
            result_nfts.push(nft.clone());
        }
    }
    result_nfts = result_nfts.iter().take(limit).cloned().collect::<Vec<_>>();

    Ok(result_nfts)
}

fn query_total_pending_reward(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    let campaign_info: CampaignInfo = CAMPAIGN_INFO.load(deps.storage)?;

    // total = pending in nft + pending in staker
    let mut total_pending_reward: Uint128 = Uint128::zero();

    // max time to calc = campaign_info.end_time
    let mut current_time = env.block.time.seconds();
    if campaign_info.end_time < env.block.time.seconds() {
        current_time = campaign_info.end_time;
    }
    // update pending reward for all nft
    let terms = campaign_info.clone().lockup_term;
    let time_calc_nft = campaign_info.time_calc_nft;
    let reward_per_second = campaign_info.reward_per_second;

    // load nfts
    let mut nfts_load = Vec::new();
    let nfts_storage = NFTS.range(deps.storage, None, None, Order::Ascending);
    for item in nfts_storage {
        let (_, nft_info) = item?;
        nfts_load.push(nft_info);
    }

    for term in terms {
        let mut nft_list = nfts_load
            .clone()
            .into_iter()
            .filter(|nft| nft.lockup_term.value == term.value)
            .collect::<Vec<_>>();
        nft_list.sort_by(|a, b| a.end_time.cmp(&b.end_time));

        let mut time_calc: u64 = time_calc_nft;
        let mut nft_count = nft_list
            .clone()
            .into_iter()
            .filter(|nft| !nft.is_end_reward)
            .collect::<Vec<_>>()
            .len() as u128;
        let mut reward = Uint128::zero();
        for nft in nft_list.iter_mut() {
            if !nft.is_end_reward {
                if nft.end_time <= current_time {
                    // calc in time_calc -> nft.end_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        nft.end_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // increase reward for next nft
                    reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, reward).unwrap();

                    nft_count -= 1; // update count nft for next calc reward
                    time_calc = nft.end_time; // update time_calc
                    nft.is_end_reward = true; // nft stake timeout
                } else {
                    // calc in time_calc -> current_time
                    let calc_reward = calc_reward_in_time(
                        time_calc,
                        current_time,
                        reward_per_second,
                        term.percent,
                        nft_count,
                    )
                    .unwrap();

                    // add reward previous and current reward
                    let accumulate_reward = add_reward(reward, calc_reward).unwrap();

                    // update reward for nft
                    nft.pending_reward = add_reward(nft.pending_reward, accumulate_reward).unwrap()
                }
            }
            // pending reward in nft
            total_pending_reward = add_reward(total_pending_reward, nft.pending_reward).unwrap();
        }
    }
    // get pending reward in staker
    let stakers_info = STAKERS_INFO.range(deps.storage, None, None, Order::Ascending);
    for item in stakers_info {
        let (_, value) = item?;
        total_pending_reward = add_reward(total_pending_reward, value.reward_debt).unwrap();
    }

    Ok(total_pending_reward)
}

fn query_token_ids(deps: Deps) -> Result<Vec<String>, ContractError> {
    let token_ids = TOKEN_IDS.load(deps.storage)?;

    Ok(token_ids)
}
