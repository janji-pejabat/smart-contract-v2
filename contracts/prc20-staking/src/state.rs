use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub paused: bool,
    pub next_room_id: u64,
}

#[cw_serde]
pub enum AssetInfo {
    Cw20(Addr),
    Native(String),
}

#[cw_serde]
pub struct StakeConfig {
    pub stake_tokens: Vec<Addr>,
    pub is_and_rule: bool, // true: must stake all tokens, false: can stake any
    pub min_stake_amount: Uint128,
}

#[cw_serde]
pub struct RewardConfig {
    pub reward_token: AssetInfo,
    pub emission_per_second: Uint128,
    pub total_deposited: Uint128,
    pub total_claimed: Uint128,
    pub acc_reward_per_share: Decimal,
}

#[cw_serde]
pub struct NFTConfig {
    pub nft_address: Addr,
    pub required_for_staking: bool,
    pub tier_multipliers: Vec<NFTTierMultiplier>,
}

#[cw_serde]
pub struct NFTTierMultiplier {
    pub tier_name: String,
    pub multiplier: Decimal,
    pub auto_compound_unlocked: bool,
}

#[cw_serde]
pub struct AutoCompoundConfig {
    pub enabled: bool,
    pub min_stake_threshold: Uint128,
    pub nft_required: bool,
}

#[cw_serde]
pub struct Room {
    pub id: u64,
    pub name: String,
    pub stake_config: StakeConfig,
    pub reward_configs: Vec<RewardConfig>,
    pub nft_config: Option<NFTConfig>,
    pub auto_compound_config: AutoCompoundConfig,
    pub total_staked_weight: Uint128,
    pub last_update_time: u64,
    pub early_withdraw_penalty: Decimal,
    pub cooldown_period: u64,
    pub paused: bool,
}

#[cw_serde]
pub struct UserPosition {
    pub room_id: u64,
    pub user: Addr,
    pub staked_amounts: Vec<(Addr, Uint128)>, // (Token, Amount)
    pub last_reward_per_share: Vec<(AssetInfo, Decimal)>,
    pub pending_rewards: Vec<(AssetInfo, Uint128)>,
    pub staked_at: u64,
    pub last_interaction: u64,
    pub auto_compound_enabled: bool,
    pub nft_multiplier: Decimal,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ROOMS: Map<u64, Room> = Map::new("rooms");
pub const USER_POSITIONS: Map<(&Addr, u64), UserPosition> = Map::new("user_positions");
