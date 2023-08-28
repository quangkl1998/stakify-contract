use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::state::{
    AssetToken, CampaignInfo, CampaignInfoUpdate, LockupTerm, NftInfo, NftStake, StakedInfoResult,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String, // owner of campaign
    // info detail
    pub campaign_name: String,
    pub campaign_image: String,
    pub campaign_description: String,

    pub limit_per_staker: u64,
    pub reward_token_info: AssetToken, // reward token
    pub allowed_collection: String,    // staking collection nft
    pub lockup_term: Vec<LockupTerm>,  // flexible, 15days, 30days, 60days

    pub start_time: u64, // start time must be from T + 1
    pub end_time: u64,   // max 3 years
}

#[cw_serde]
pub enum ExecuteMsg {
    AddRewardToken {
        amount: Uint128,
    },
    // user can stake 1 or many nfts to this campaign
    StakeNfts {
        nfts: Vec<NftStake>,
    },

    // user can claim reward
    ClaimReward {
        amount: Uint128,
    },

    WithdrawReward {},

    UnStakeNft {
        token_id: String,
    },

    // update campaign
    UpdateCampaign {
        campaign_info_update: CampaignInfoUpdate,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(CampaignInfo)]
    CampaignInfo {},

    #[returns(NftInfo)]
    NftInfo { token_id: String },

    #[returns(NftInfo)]
    Nft { token_id: String },

    #[returns(StakedInfoResult)]
    NftStaked { owner: Addr },

    #[returns(Vec<NftInfo>)]
    Nfts {
        limit: Option<u32>,
    },

    #[returns(Uint128)]
    TotalPendingReward {},

    #[returns(Vec<String>)]
    TokenIds {},
}
