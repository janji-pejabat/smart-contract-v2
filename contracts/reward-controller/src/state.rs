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
    pub referral_commission_bps: u16,
    pub batch_limit: u32,
}

#[cw_serde]
pub enum AssetInfo {
    Cw20(Addr),
    Native(String),
}

impl AssetInfo {
    pub fn to_key(&self) -> String {
        match self {
            AssetInfo::Cw20(addr) => format!("c:{}", addr),
            AssetInfo::Native(denom) => format!("n:{}", denom),
        }
    }

    pub fn from_key(key: String) -> Self {
        let parts: Vec<&str> = key.splitn(2, ':').collect();
        match parts[0] {
            "c" => AssetInfo::Cw20(Addr::unchecked(parts[1])),
            _ => AssetInfo::Native(parts[1].to_string()),
        }
    }
}

#[cw_serde]
pub struct DynamicAPRConfig {
    pub base_apr: Decimal,
    pub tvl_threshold_low: Uint128,
    pub tvl_threshold_high: Uint128,
    pub adjustment_factor: Decimal, // e.g. 0.2 for 20% boost/reduction
}

#[cw_serde]
pub struct RewardPool {
    pub pool_id: u64,
    pub lp_token: Addr,
    pub reward_token: AssetInfo,
    pub total_deposited: Uint128,
    pub total_claimed: Uint128,
    pub apr: Decimal, // Active APR
    pub dynamic_config: Option<DynamicAPRConfig>,
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
    pub locked_at: u64,
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
pub const REFERRALS: Map<&Addr, Addr> = Map::new("referrals"); // referee -> referrer
pub const REFERRER_BALANCES: Map<(&Addr, String), Uint128> = Map::new("referrer_balances");
