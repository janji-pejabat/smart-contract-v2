use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use crate::state::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub lp_locker_contract: String,
    pub claim_interval: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    RegisterStake {
        locker_id: u64,
    },
    UnregisterStake {
        locker_id: u64,
    },
    ClaimRewards {
        pool_ids: Vec<u64>,
    },
    CreateRewardPool {
        reward_token: AssetInfo,
        emission_per_second: Uint128,
        start_time: u64,
        end_time: Option<u64>,
    },
    UpdateRewardPool {
        pool_id: u64,
        emission_per_second: Option<Uint128>,
        end_time: Option<u64>,
        enabled: Option<bool>,
    },
    DepositRewards {
        pool_id: u64,
    },
    WithdrawRewards {
        pool_id: u64,
        amount: Uint128,
    },
    UpdateConfig {
        admin: Option<String>,
        lp_locker_contract: Option<String>,
        claim_interval: Option<u64>,
    },
    Pause {},
    Resume {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(RewardPoolResponse)]
    RewardPool { pool_id: u64 },

    #[returns(Vec<RewardPoolResponse>)]
    AllRewardPools {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(UserStakeResponse)]
    UserStake {
        user: String,
        locker_id: u64,
    },

    #[returns(PendingRewardsResponse)]
    PendingRewards {
        user: String,
        pool_id: u64,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub lp_locker_contract: Addr,
    pub paused: bool,
    pub claim_interval: u64,
    pub next_pool_id: u64,
}

#[cw_serde]
pub struct RewardPoolResponse {
    pub pool_id: u64,
    pub reward_token: AssetInfo,
    pub total_deposited: Uint128,
    pub total_claimed: Uint128,
    pub emission_per_second: Uint128,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub enabled: bool,
}

#[cw_serde]
pub struct UserStakeResponse {
    pub user: Addr,
    pub locker_id: u64,
    pub lp_amount: Uint128,
    pub lock_start: u64,
    pub lock_duration: u64,
    pub bonus_multiplier: Decimal,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub pool_id: u64,
    pub pending_amount: Uint128,
}
