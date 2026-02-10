use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub reward_controller: Option<Addr>,
    pub emergency_unlock_delay: u64,
    pub platform_fee_bps: u16,
    pub paused: bool,
    pub next_locker_id: u64,
}

#[cw_serde]
pub struct Locker {
    pub id: u64,
    pub owner: Addr,
    pub lp_token: Addr,
    pub amount: Uint128,
    pub locked_at: u64,
    pub unlock_time: u64,
    pub extended_count: u8,
    pub emergency_unlock_requested: Option<u64>,
    pub metadata: Option<String>,
}

#[cw_serde]
pub struct WhitelistedLP {
    pub lp_token: Addr,
    pub min_lock_duration: u64,
    pub max_lock_duration: u64,
    pub enabled: bool,
    pub bonus_multiplier: Decimal,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const LOCKERS: Map<u64, Locker> = Map::new("lockers");
pub const USER_LOCKERS: Map<(&Addr, u64), bool> = Map::new("user_lockers");
pub const WHITELISTED_LPS: Map<&Addr, WhitelistedLP> = Map::new("whitelisted_lps");
pub const TOTAL_LOCKED: Map<&Addr, Uint128> = Map::new("total_locked");
