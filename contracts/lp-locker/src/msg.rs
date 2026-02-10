use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub emergency_unlock_delay: u64, // seconds, default 259200 (3 days)
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receive CW20 tokens (LP tokens to lock)
    Receive(Cw20ReceiveMsg),

    /// Unlock LP tokens after unlock_time
    UnlockLP { locker_id: u64 },

    /// Batch unlock multiple LP tokens
    BatchUnlock { locker_ids: Vec<u64> },

    /// Extend lock duration
    ExtendLock {
        locker_id: u64,
        new_unlock_time: u64,
    },

    /// Batch extend multiple lock durations
    BatchExtendLock { locks: Vec<(u64, u64)> },

    /// Request emergency unlock (starts delay timer)
    RequestEmergencyUnlock { locker_id: u64 },

    /// Execute emergency unlock (after delay)
    ExecuteEmergencyUnlock { locker_id: u64 },

    /// Admin: Update configuration
    UpdateConfig {
        admin: Option<String>,
        reward_controller: Option<String>,
        emergency_unlock_delay: Option<u64>,
        platform_fee_bps: Option<u16>,
        batch_limit: Option<u32>,
    },

    /// Admin: Whitelist LP token
    WhitelistLP {
        lp_token: String,
        name: String,
        symbol: String,
        min_lock_duration: u64,
        max_lock_duration: u64,
        bonus_multiplier: Decimal,
    },

    /// Admin: Remove LP from whitelist
    RemoveLP { lp_token: String },

    /// Admin: Pause contract
    Pause {},

    /// Admin: Resume contract
    Resume {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Lock LP tokens
    LockLP {
        unlock_time: u64,
        metadata: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(LockerResponse)]
    Locker { locker_id: u64 },

    #[returns(LockersResponse)]
    LockersByOwner {
        owner: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(WhitelistedLPResponse)]
    WhitelistedLP { lp_token: String },

    #[returns(Vec<WhitelistedLPResponse>)]
    AllWhitelistedLPs {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(TotalLockedResponse)]
    TotalLockedByLP { lp_token: String },
}

// Response types
#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub reward_controller: Option<Addr>,
    pub emergency_unlock_delay: u64,
    pub platform_fee_bps: u16,
    pub batch_limit: u32,
    pub paused: bool,
    pub next_locker_id: u64,
}

#[cw_serde]
pub struct LockerResponse {
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
pub struct LockersResponse {
    pub lockers: Vec<LockerResponse>,
}

#[cw_serde]
pub struct WhitelistedLPResponse {
    pub lp_token: Addr,
    pub name: String,
    pub symbol: String,
    pub min_lock_duration: u64,
    pub max_lock_duration: u64,
    pub enabled: bool,
    pub bonus_multiplier: Decimal,
    pub total_locked_all_time: Uint128,
    pub total_unlocked_all_time: Uint128,
    pub user_count: u64,
}

#[cw_serde]
pub struct TotalLockedResponse {
    pub lp_token: Addr,
    pub total_amount: Uint128,
}

#[cw_serde]
pub enum MigrateMsg {
    V1ToV2 { reward_controller: Option<String> },
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
