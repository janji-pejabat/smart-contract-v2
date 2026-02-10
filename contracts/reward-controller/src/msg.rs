use crate::state::AssetInfo;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub lp_locker_contract: String,
    pub claim_interval: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receive CW20 tokens (for reward deposits)
    Receive(Cw20ReceiveMsg),

    /// Receive notifications from LP Locker
    LockerHook(LockerHookMsg),

    ClaimRewards {
        locker_id: u64,
        pool_ids: Vec<u64>,
    },
    CreateRewardPool {
        lp_token: String,
        reward_token: AssetInfo,
        apr: Decimal,
    },
    UpdateRewardPool {
        pool_id: u64,
        apr: Option<Decimal>,
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
    UserStake { user: String, locker_id: u64 },

    #[returns(PendingRewardsResponse)]
    PendingRewards { user: String, pool_id: u64 },
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
pub enum Cw20HookMsg {
    DepositRewards { pool_id: u64 },
}

#[cw_serde]
pub enum LockerHookMsg {
    OnLock {
        locker_id: u64,
        owner: String,
        lp_token: String,
        amount: Uint128,
        locked_at: u64,
        unlock_time: u64,
    },
    OnExtend {
        locker_id: u64,
        new_unlock_time: u64,
    },
    OnUnlock {
        locker_id: u64,
        owner: String,
    },
}

#[cw_serde]
pub struct RewardPoolResponse {
    pub pool_id: u64,
    pub lp_token: Addr,
    pub reward_token: AssetInfo,
    pub total_deposited: Uint128,
    pub total_claimed: Uint128,
    pub apr: Decimal,
    pub enabled: bool,
}

#[cw_serde]
pub struct UserStakeResponse {
    pub user: Addr,
    pub locker_id: u64,
    pub lp_token: Addr,
    pub lp_amount: Uint128,
    pub bonus_multiplier: Decimal,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub pool_id: u64,
    pub pending_amount: Uint128,
}
