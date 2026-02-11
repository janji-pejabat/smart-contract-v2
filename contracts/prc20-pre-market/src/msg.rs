use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub platform_fee_bps: u64,
    pub fee_receiver: String,
    pub native_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Hook for CW20 tokens to create a listing
    Receive(Cw20ReceiveMsg),
    /// Buy tokens from a listing
    Buy {
        listing_id: u64,
        amount: Uint128,
        referrer: Option<String>,
    },
    /// Cancel a listing (seller only)
    CancelListing { listing_id: u64 },
    /// Pause a listing (seller only)
    PauseListing { listing_id: u64 },
    /// Resume a listing (seller only)
    ResumeListing { listing_id: u64 },
    /// Update metadata (seller only)
    UpdateListingMetadata { listing_id: u64, metadata: String },
    /// Update configuration (admin only)
    UpdateConfig {
        admin: Option<String>,
        platform_fee_bps: Option<u64>,
        fee_receiver: Option<String>,
        paused: Option<bool>,
    },
    /// Blacklist a token (admin only)
    SetTokenBlacklist {
        token_address: String,
        blacklisted: bool,
    },
}

#[cw_serde]
pub struct RoundConfig {
    pub name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub price_per_token: Uint128,
    pub max_wallet_limit: Option<Uint128>,
    pub whitelist: Option<Vec<String>>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    CreateListing {
        min_buy: Option<Uint128>,
        max_buy: Option<Uint128>,
        rounds: Vec<RoundConfig>,
        metadata: String,
        royalty_address: Option<String>,
        royalty_bps: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    #[returns(crate::state::Listing)]
    Listing { id: u64 },
    #[returns(Vec<crate::state::Listing>)]
    Listings {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Vec<crate::state::Listing>)]
    ListingsBySeller {
        seller: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Vec<crate::state::Listing>)]
    ListingsByToken {
        token: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Vec<crate::state::Purchase>)]
    BuyerPurchaseHistory {
        buyer: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(crate::state::GlobalStats)]
    Stats {},
}

#[cw_serde]
pub struct MigrateMsg {}
