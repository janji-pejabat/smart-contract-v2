use crate::msg::VestingSchedule;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub paused: bool,
}

#[cw_serde]
pub struct VestingAccount {
    pub id: u64,
    pub beneficiary: Addr,
    pub token_address: Addr,
    pub total_amount: Uint128,
    pub released_amount: Uint128,
    pub revoked: bool,
    pub category: String,
    pub revocable: bool,
    pub schedule: VestingSchedule,
}

#[cw_serde]
pub struct GlobalStats {
    pub total_vested: Uint128,
    pub total_claimed: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const VESTING_ACCOUNTS: Map<u64, VestingAccount> = Map::new("vesting_accounts");
pub const VESTING_COUNT: Item<u64> = Item::new("vesting_count");
pub const GLOBAL_STATS: Map<&Addr, GlobalStats> = Map::new("global_stats");

// Indexes for queries
// (Beneficiary, ID) -> bool
pub const BENEFICIARY_VESTINGS: Map<(&Addr, u64), bool> = Map::new("beneficiary_vestings");
// (Category, ID) -> bool
pub const CATEGORY_VESTINGS: Map<(&str, u64), bool> = Map::new("category_vestings");
