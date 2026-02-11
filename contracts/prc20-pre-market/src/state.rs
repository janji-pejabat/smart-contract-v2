use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub platform_fee_bps: u64,
    pub fee_receiver: Addr,
    pub native_denom: String,
    pub paused: bool,
}

#[cw_serde]
pub enum ListingStatus {
    Active,
    Paused,
    Sold,
    Cancelled,
}

#[cw_serde]
pub struct Round {
    pub name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub price_per_token: Uint128,
    pub max_wallet_limit: Option<Uint128>,
    pub whitelist: Option<Vec<Addr>>,
}

#[cw_serde]
pub struct Listing {
    pub id: u64,
    pub seller: Addr,
    pub token_address: Addr,
    pub total_amount: Uint128,
    pub remaining_amount: Uint128,
    pub min_buy: Uint128,
    pub max_buy: Uint128,
    pub rounds: Vec<Round>,
    pub status: ListingStatus,
    pub metadata: String,
    pub royalty_address: Option<Addr>,
    pub royalty_bps: u64,
}

#[cw_serde]
pub struct GlobalStats {
    pub total_volume_paxi: Uint128,
    pub total_trades: u64,
    pub total_fees_collected: Uint128,
}

#[cw_serde]
pub struct Purchase {
    pub id: u64,
    pub listing_id: u64,
    pub buyer: Addr,
    pub amount: Uint128,
    pub total_paid: Uint128,
    pub timestamp: u64,
}

pub struct ListingIndexes<'a> {
    pub seller: MultiIndex<'a, Addr, Listing, u64>,
    pub token: MultiIndex<'a, Addr, Listing, u64>,
}

impl<'a> IndexList<Listing> for ListingIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Listing>> + '_> {
        let v: Vec<&dyn Index<Listing>> = vec![&self.seller, &self.token];
        Box::new(v.into_iter())
    }
}

pub fn listings<'a>() -> IndexedMap<'a, u64, Listing, ListingIndexes<'a>> {
    let indexes = ListingIndexes {
        seller: MultiIndex::new(|_pk, l| l.seller.clone(), "listings", "listings__seller"),
        token: MultiIndex::new(
            |_pk, l| l.token_address.clone(),
            "listings",
            "listings__token",
        ),
    };
    IndexedMap::new("listings", indexes)
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATS: Item<GlobalStats> = Item::new("stats");
pub const TOKEN_BLACKLIST: Map<&Addr, bool> = Map::new("token_blacklist");

pub const USER_PURCHASES: Map<(Addr, u64), Purchase> = Map::new("user_purchases");
pub const NEXT_PURCHASE_ID: Item<u64> = Item::new("next_purchase_id");
pub const USER_TOTAL_BOUGHT: Map<(u64, Addr), Uint128> = Map::new("user_total_bought");

pub const NEXT_LISTING_ID: Item<u64> = Item::new("next_listing_id");
