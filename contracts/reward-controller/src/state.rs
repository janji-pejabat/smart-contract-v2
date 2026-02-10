use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct RewardConfig {
    pub admin: Addr,
    pub lp_locker_contract: Addr,
    pub paused: bool,
    pub claim_interval: u64,
    pub next_pool_id: u64,
}

#[cw_serde]
pub enum AssetInfo {
    Cw20(Addr),
    Native(String),
}

#[cw_serde]
pub struct RewardPool {
    pub pool_id: u64,
    pub reward_token: AssetInfo,
    pub total_deposited: Uint128,
    pub total_claimed: Uint128,
    pub emission_per_second: Uint128,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub last_update: u64,
    pub reward_per_token_stored: Decimal,
    pub enabled: bool,
}

#[cw_serde]
pub struct UserStake {
    pub user: Addr,
    pub locker_id: u64,
    pub lp_amount: Uint128,
    pub lock_start: u64,
    pub lock_duration: u64,
    pub bonus_multiplier: Decimal,
}

#[cw_serde]
pub struct UserReward {
    pub user: Addr,
    pub pool_id: u64,
    pub reward_per_token_paid: Decimal,
    pub rewards_accrued: Uint128,
    pub last_claim_time: u64,
}

pub const CONFIG: Item<RewardConfig> = Item::new("config");
pub const POOLS: Map<u64, RewardPool> = Map::new("pools");
pub const USER_STAKES: Map<(&Addr, u64), UserStake> = Map::new("user_stakes");
pub const USER_REWARDS: Map<(&Addr, u64), UserReward> = Map::new("user_rewards");
pub const TOTAL_STAKED: Item<Uint128> = Item::new("total_staked");
