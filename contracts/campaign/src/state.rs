use std::fmt;

use cosmwasm_schema::cw_serde; // attribute macro to (de)serialize and make schemas
use cosmwasm_std::{Addr, Uint128}; // address type
use cw_storage_plus::{Item, Map}; // analog of Singletons for storage

#[cw_serde]
pub enum TokenInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

impl fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenInfo::NativeToken { denom } => write!(f, "{}", denom),
            TokenInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}

#[cw_serde]
pub struct AssetToken {
    pub info: TokenInfo,
    pub amount: Uint128,
}

impl fmt::Display for AssetToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.info, self.amount)
    }
}

pub enum Term {
    _15days,
    _30days,
    _60days,
}

impl Term {
    pub fn from_value(s: &u64) -> Option<Self> {
        match s {
            1296000 => Some(Term::_15days), // 86400 * 15
            2592000 => Some(Term::_30days), // 86400 * 30
            5184000 => Some(Term::_60days), // 86400 * 60
            _ => None,
        }
    }
}

#[cw_serde]
pub struct LockupTerm {
    pub value: u64,
    pub percent: Uint128,
}

impl fmt::Display for LockupTerm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.value, self.percent)
    }
}

#[cw_serde]
pub struct CampaignInfo {
    pub owner: Addr, // owner of campaign
    // info detail
    pub campaign_name: String,
    pub campaign_image: String,
    pub campaign_description: String,
    pub total_reward_claimed: Uint128, // default 0
    pub total_reward: Uint128,         // default 0
    pub limit_per_staker: u64,         // max nft can stake
    pub reward_token: AssetToken,      // reward token
    pub allowed_collection: Addr,      // staking collection nft
    pub lockup_term: Vec<LockupTerm>,  // 15days, 30days, 60days
    pub reward_per_second: Uint128,
    pub time_calc_nft: u64,
    pub start_time: u64, // start time must be from T + 1
    pub end_time: u64,   // max 3 years
}

pub enum UpdateCampaign {
    UpdateTimeCalc(u64),
}

// impl CampaignInfo {
//     pub fn update(&mut self, action: UpdateCampaign) {
//         match action {
//             UpdateCampaign::UpdateTimeCalc(new_time) => self.time_calc_nft = new_time,
//         }
//     }
// }

#[cw_serde]
pub struct StakerRewardAssetInfo {
    pub token_ids: Vec<String>,
    pub reward_debt: Uint128, // can claim reward.
    pub reward_claimed: Uint128,
}

#[cw_serde]
pub struct NftInfo {
    pub token_id: String,
    pub owner: Addr,
    pub pending_reward: Uint128,
    pub lockup_term: LockupTerm, // value = seconds
    pub is_end_reward: bool,
    pub start_time: u64,
    pub end_time: u64,
}

#[cw_serde]
pub struct NftStake {
    pub token_id: String,
    pub lockup_term: u64,
}

// campaign info
pub const CAMPAIGN_INFO: Item<CampaignInfo> = Item::new("campaign_info");

// Mapping from staker address to staked nft.
pub const STAKERS_INFO: Map<Addr, StakerRewardAssetInfo> = Map::new("stakers_info");

pub const STAKERS: Map<u64, Addr> = Map::new("staker");

// list token_id nft
pub const TOKEN_IDS: Item<Vec<String>> = Item::new("token_ids");

// list nft staked
pub const NFTS: Map<String, NftInfo> = Map::new("nfts");

// result query
#[cw_serde]
pub struct CampaignInfoResult {
    pub owner: Addr,
    pub campaign_name: String,
    pub campaign_image: String,
    pub campaign_description: String,
    pub total_nft_staked: u64,
    pub total_reward_claimed: Uint128,
    pub total_reward: Uint128,
    pub limit_per_staker: u64,
    pub reward_token_info: AssetToken,
    pub allowed_collection: Addr,
    pub lockup_term: Vec<LockupTerm>,
    pub reward_per_second: Uint128,
    pub time_calc_nft: u64,
    pub start_time: u64,
    pub end_time: u64,
}

#[cw_serde]
pub struct StakedInfoResult {
    pub nfts: Vec<NftInfo>,
    pub reward_debt: Uint128, // can claim reward.
    pub reward_claimed: Uint128,
}

#[cw_serde]
pub struct CampaignInfoUpdate {
    pub campaign_name: Option<String>,
    pub campaign_image: Option<String>,
    pub campaign_description: Option<String>,
    pub limit_per_staker: Option<u64>,
    pub lockup_term: Option<Vec<LockupTerm>>,
    pub start_time: Option<u64>, // start time must be from T + 1
    pub end_time: Option<u64>,   // max 3 years
}
