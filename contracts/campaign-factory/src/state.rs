use campaign::state::TokenInfo;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub campaign_code_id: u64,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub campaign_code_id: u64,
}

#[cw_serde]
pub struct FactoryCampaign {
    pub owner: Addr,
    pub campaign_addr: Addr,
    pub reward_token: TokenInfo,
    pub allowed_collection: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const CAMPAIGNS: Map<u64, FactoryCampaign> = Map::new("campaigns");
pub const NUMBER_OF_CAMPAIGNS: Item<u64> = Item::new("number_of_campaigns");
pub const ADDR_CAMPAIGNS: Item<Vec<String>> = Item::new("addr_campaigns");

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
    /// This is how much the minter takes as a cut when sold
    /// royalties are owed on this token if it is Some
    pub royalty_percentage: Option<u64>,
    /// The payment address, may be different to or the same
    /// as the minter addr
    /// question: how do we validate this?
    pub royalty_payment_address: Option<String>,
}
