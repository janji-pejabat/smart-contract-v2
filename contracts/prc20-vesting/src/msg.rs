use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receive CW20 tokens to create a vesting schedule
    Receive(Cw20ReceiveMsg),
    /// Claim vested tokens for specific vesting IDs
    Claim { ids: Vec<u64> },
    /// Revoke a revocable vesting schedule
    Revoke { id: u64 },
    /// Update the admin address
    UpdateAdmin { admin: String },
    /// Pause/Unpause the contract
    SetPaused { paused: bool },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Create a new vesting schedule.
    /// The total_amount is taken from the Cw20ReceiveMsg.amount
    CreateVesting {
        beneficiary: String,
        schedule: VestingSchedule,
        category: String,
        revocable: bool,
    },
    /// Create multiple vesting schedules from a single CW20 transfer
    BatchCreateVesting { vestings: Vec<VestingCreation> },
}

#[cw_serde]
pub struct VestingCreation {
    pub beneficiary: String,
    pub amount: Uint128,
    pub schedule: VestingSchedule,
    pub category: String,
    pub revocable: bool,
}

#[cw_serde]
pub enum VestingSchedule {
    /// Linear vesting with optional cliff
    Linear {
        start_time: u64,
        end_time: u64,
        cliff_time: Option<u64>,
        release_interval: u64,
    },
    /// Custom milestones
    Custom { milestones: Vec<Milestone> },
}

#[cw_serde]
pub struct Milestone {
    pub timestamp: u64,
    pub amount: Uint128,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(VestingResponse)]
    Vesting { id: u64 },
    #[returns(Vec<VestingResponse>)]
    VestingsByBeneficiary {
        beneficiary: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Vec<VestingResponse>)]
    VestingsByCategory {
        category: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Uint128)]
    ClaimableAmount { id: u64 },
    #[returns(GlobalStatsResponse)]
    GlobalStats { token_address: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub paused: bool,
}

#[cw_serde]
pub struct VestingResponse {
    pub id: u64,
    pub beneficiary: Addr,
    pub token_address: Addr,
    pub total_amount: Uint128,
    pub released_amount: Uint128,
    pub revoked: bool,
    pub category: String,
    pub revocable: bool,
    pub schedule: VestingSchedule,
    pub claimable_amount: Uint128,
}

#[cw_serde]
pub struct GlobalStatsResponse {
    pub total_vested: Uint128,
    pub total_claimed: Uint128,
    pub active_vesting_count: u64,
}
