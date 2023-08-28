use crate::error::ContractError;
use crate::state::{
    Config, ConfigResponse, FactoryCampaign, ADDR_CAMPAIGNS, CONFIG, NUMBER_OF_CAMPAIGNS,
};
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::CAMPAIGNS,
};
// use campaign::msg::ExecuteMsg as CampaignExecuteMsg;
use campaign::msg::InstantiateMsg as CampaignInstantiateMsg;
use campaign::msg::QueryMsg as CampaignQueryMsg;
use campaign::state::{AssetToken, CampaignInfoResult, LockupTerm};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:campaign-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: info.sender,
        campaign_code_id: msg.campaign_code_id,
    };

    // init NUMBER_OF_CAMPAIGNS to 0
    NUMBER_OF_CAMPAIGNS.save(deps.storage, &0u64)?;

    // init ADDR_CAMPAIGNS to vec![]
    ADDR_CAMPAIGNS.save(deps.storage, &vec![])?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            campaign_code_id,
        } => execute_update_config(deps, env, info, owner, campaign_code_id),
        ExecuteMsg::CreateCampaign {
            owner,
            campaign_name,
            campaign_image,
            campaign_description,
            start_time,
            end_time,
            limit_per_staker,
            reward_token_info,
            allowed_collection,
            lockup_term,
        } => execute_create_campaign(
            deps,
            env,
            info,
            owner,
            campaign_name,
            campaign_image,
            campaign_description,
            start_time,
            end_time,
            limit_per_staker,
            reward_token_info,
            allowed_collection,
            lockup_term,
        ),
    }
}

// Only owner can execute it
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    campaign_code_id: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update owner if provided
    if let Some(owner) = owner.clone() {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    // update campaign_code_id if provided
    if let Some(campaign_code_id) = campaign_code_id {
        config.campaign_code_id = campaign_code_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("owner", owner.unwrap())
        .add_attribute("campaign_code_id", campaign_code_id.unwrap().to_string()))
}

// Anyone can execute it to create a new pool
#[allow(clippy::too_many_arguments)]
pub fn execute_create_campaign(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    owner: String,
    campaign_name: String,
    campaign_image: String,
    campaign_description: String,
    start_time: u64,
    end_time: u64,
    limit_per_staker: u64,
    reward_token_info: AssetToken,
    allowed_collection: String,
    lockup_term: Vec<LockupTerm>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    // // get current time
    // let current_time = env.block.time.seconds();

    // // Not allow start time is greater than end time
    // if start_time >= end_time {
    //     return Err(ContractError::Std(StdError::generic_err(
    //         "## Start time is greater than end time ##",
    //     )));
    // }

    // // Not allow to create a campaign when current time is greater than start time
    // if current_time > start_time {
    //     return Err(ContractError::Std(StdError::generic_err(
    //         "## Current time is greater than start time ##",
    //     )));
    // }

    Ok(Response::new()
        .add_attributes(vec![
            ("method", "create_campaign"),
            ("campaign_owner", owner.as_str()),
            ("campaign_name", campaign_name.as_str()),
            ("campaign_image", campaign_image.as_str()),
            ("campaign_description", campaign_description.as_str()),
            ("start_time", start_time.to_string().as_str()),
            ("end_time", end_time.to_string().as_str()),
            ("limit_per_staker", limit_per_staker.to_string().as_str()),
            ("reward_token_info", &format!("{}", reward_token_info)),
            ("allowed_collection", allowed_collection.as_str()),
            ("lockup_term", &format!("{:?}", &lockup_term)),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.campaign_code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label: "pair".to_string(),
                msg: to_binary(&CampaignInstantiateMsg {
                    owner,
                    campaign_name,
                    campaign_image,
                    campaign_description,
                    limit_per_staker,
                    reward_token_info,
                    allowed_collection,
                    lockup_term,
                    start_time,
                    end_time,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let reply = parse_reply_instantiate_data(msg).unwrap();

    let campaign_contract = &reply.contract_address;
    let campaign_info: CampaignInfoResult =
        query_pair_info_from_pair(&deps.querier, Addr::unchecked(campaign_contract))?;

    let campaign_key = NUMBER_OF_CAMPAIGNS.load(deps.storage)? + 1;

    CAMPAIGNS.save(
        deps.storage,
        campaign_key,
        &FactoryCampaign {
            owner: campaign_info.owner.clone(),
            campaign_addr: deps.api.addr_validate(campaign_contract)?,
            reward_token: campaign_info.reward_token_info.info,
            allowed_collection: campaign_info.allowed_collection,
        },
    )?;

    // increase campaign count
    NUMBER_OF_CAMPAIGNS.save(deps.storage, &(campaign_key))?;

    let mut addr_campaigns = ADDR_CAMPAIGNS.load(deps.storage)?;
    addr_campaigns.push(campaign_contract.clone());
    ADDR_CAMPAIGNS.save(deps.storage, &addr_campaigns)?;

    Ok(Response::new().add_attributes([
        ("action", "reply_on_create_campaign_success"),
        ("campaign_key", campaign_key.to_string().as_str()),
        ("campaign_contract_addr", campaign_contract),
        ("owner", campaign_info.owner.as_ref()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Campaign { campaign_id } => to_binary(&query_campaign_info(deps, campaign_id)?),
        QueryMsg::Campaigns { start_after, limit } => {
            to_binary(&query_campaigns(deps, start_after, limit)?)
        }
        QueryMsg::CampaignAddrs {} => to_binary(&query_addr_campaigns(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        campaign_code_id: state.campaign_code_id,
    };

    Ok(resp)
}

pub fn query_campaign_info(deps: Deps, campaign_id: u64) -> StdResult<FactoryCampaign> {
    let campaign_info = CAMPAIGNS.load(deps.storage, campaign_id)?;
    Ok(campaign_info)
}

pub fn query_campaigns(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<FactoryCampaign>> {
    let start_after = start_after.unwrap_or(0);
    let limit = limit.unwrap_or(30) as usize;
    let campaign_count = NUMBER_OF_CAMPAIGNS.load(deps.storage)?;

    let campaigns = (start_after..campaign_count)
        .map(|pool_id| CAMPAIGNS.load(deps.storage, pool_id + 1))
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(campaigns)
}

pub fn query_addr_campaigns(deps: Deps) -> StdResult<Vec<String>> {
    let addr_campaigns = ADDR_CAMPAIGNS.load(deps.storage)?;
    Ok(addr_campaigns)
}

fn query_pair_info_from_pair(
    querier: &QuerierWrapper,
    pair_contract: Addr,
) -> StdResult<CampaignInfoResult> {
    let pair_info: CampaignInfoResult = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&CampaignQueryMsg::CampaignInfo {})?,
    }))?;

    Ok(pair_info)
}
