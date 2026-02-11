use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use crate::state::{AssetInfo, StakeConfig, NFTConfig, AutoCompoundConfig, Room, UserPosition};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Stake PRC20 tokens via CW20 Receive
    Receive(Cw20ReceiveMsg),

    /// Unstake from a room
    Unstake {
        room_id: u64,
        amount: Uint128,
        token_address: String,
    },

    /// Claim all pending rewards from a room
    ClaimRewards {
        room_id: u64,
    },

    /// Opt-in/out of auto-compound for a room
    ToggleAutoCompound {
        room_id: u64,
        enabled: bool,
    },

    /// Compound rewards for a room manually
    Compound {
        room_id: u64,
    },

    // Admin Messages
    CreateRoom {
        name: String,
        stake_config: StakeConfig,
        nft_config: Option<NFTConfig>,
        auto_compound_config: AutoCompoundConfig,
        early_withdraw_penalty: Decimal,
        cooldown_period: u64,
    },

    UpdateRoom {
        room_id: u64,
        paused: Option<bool>,
        early_withdraw_penalty: Option<Decimal>,
        cooldown_period: Option<u64>,
    },

    AddRewardPool {
        room_id: u64,
        reward_token: AssetInfo,
        emission_per_second: Uint128,
    },

    UpdateRewardPool {
        room_id: u64,
        reward_token: AssetInfo,
        emission_per_second: Uint128,
    },

    FundRewardPool {
        room_id: u64,
    },

    UpdateConfig {
        admin: Option<String>,
        paused: Option<bool>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake { room_id: u64 },
    FundPool { room_id: u64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},

    #[returns(RoomResponse)]
    Room { room_id: u64 },

    #[returns(Vec<RoomResponse>)]
    Rooms {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(UserPositionResponse)]
    UserPosition {
        room_id: u64,
        user: String,
    },

    #[returns(PendingRewardsResponse)]
    PendingRewards {
        room_id: u64,
        user: String,
    },

    #[returns(APREstimateResponse)]
    APREstimate {
        room_id: u64,
    },

    #[returns(EligibilityResponse)]
    Eligibility {
        room_id: u64,
        user: String,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub paused: bool,
}

#[cw_serde]
pub struct RoomResponse {
    pub room: Room,
}

#[cw_serde]
pub struct UserPositionResponse {
    pub position: Option<UserPosition>,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub rewards: Vec<(AssetInfo, Uint128)>,
}

#[cw_serde]
pub struct APREstimateResponse {
    pub aprs: Vec<(AssetInfo, Decimal)>,
}

#[cw_serde]
pub struct EligibilityResponse {
    pub can_stake: bool,
    pub can_auto_compound: bool,
    pub nft_multiplier: Decimal,
}
