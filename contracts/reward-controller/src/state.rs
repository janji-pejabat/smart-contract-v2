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
    pub lp_token: Addr,
    pub reward_token: AssetInfo,
    pub total_deposited: Uint128,
    pub total_claimed: Uint128,
    pub apr: Decimal, // Annual Percentage Rate (e.g. 0.1 for 10%)
    pub last_update: u64,
    pub reward_per_token_stored: Decimal,
    pub enabled: bool,
}

#[cw_serde]
pub struct UserStake {
    pub user: Addr,
    pub locker_id: u64,
    pub lp_token: Addr,
    pub lp_amount: Uint128,
    pub bonus_multiplier: Decimal,
}

#[cw_serde]
pub struct UserReward {
    pub locker_id: u64,
    pub pool_id: u64,
    pub reward_per_token_paid: Decimal,
    pub rewards_accrued: Uint128,
    pub last_claim_time: u64,
}

pub const CONFIG: Item<RewardConfig> = Item::new("config");
pub const POOLS: Map<u64, RewardPool> = Map::new("pools");
pub const LP_POOLS: Map<&Addr, Vec<u64>> = Map::new("lp_pools");
pub const USER_STAKES: Map<u64, UserStake> = Map::new("user_stakes"); // Keyed by locker_id
pub const USER_STAKED_LOCKERS: Map<(&Addr, u64), bool> = Map::new("user_staked_lockers"); // (owner, locker_id)
pub const USER_REWARDS: Map<(u64, u64), UserReward> = Map::new("user_rewards"); // (locker_id, pool_id)
pub const TOTAL_STAKED: Map<&Addr, Uint128> = Map::new("total_staked"); // Per LP token
