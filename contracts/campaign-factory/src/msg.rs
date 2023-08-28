use crate::state::{ConfigResponse, FactoryCampaign};
use campaign::state::{AssetToken, LockupTerm};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    /// Campaign code ID
    pub campaign_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// UpdateConfig update relevant code IDs
    UpdateConfig {
        owner: Option<String>,
        campaign_code_id: Option<u64>,
    },
    /// CreateCampaign instantiates pair contract
    CreateCampaign {
        // info detail
        owner: String,
        campaign_name: String,
        campaign_image: String,
        campaign_description: String,
        start_time: u64, // start time must be from T + 1
        end_time: u64,   // max 3 years

        limit_per_staker: u64,
        // status: String, // pending | upcoming | active | ended
        reward_token_info: AssetToken, // reward token
        allowed_collection: String,    // staking collection nft
        lockup_term: Vec<LockupTerm>,  // flexible, 15days, 30days, 60days
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(FactoryCampaign)]
    Campaign { campaign_id: u64 },

    #[returns(Vec<FactoryCampaign>)]
    Campaigns {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(Vec<String>)]
    CampaignAddrs {},
}
