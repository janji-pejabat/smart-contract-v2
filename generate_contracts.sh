#!/usr/bin/env bash
# Script to generate complete contract structure

set -e

echo "Generating LP Locker contract files..."

# Create LP Locker error.rs
cat > contracts/lp-locker/src/error.rs << 'EOF'
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Contract is paused")]
    Paused {},

    #[error("LP token not whitelisted")]
    LPNotWhitelisted {},

    #[error("Locker not found")]
    LockerNotFound {},

    #[error("Not locker owner")]
    NotOwner {},

    #[error("LP still locked until {0}")]
    StillLocked(u64),

    #[error("Invalid unlock time: must be between {min} and {max} seconds")]
    InvalidUnlockTime { min: u64, max: u64 },

    #[error("New unlock time must be greater than current unlock time")]
    InvalidExtension {},

    #[error("Emergency unlock not requested")]
    EmergencyNotRequested {},

    #[error("Emergency unlock delay not passed yet (execute at: {0})")]
    EmergencyDelayNotPassed(u64),

    #[error("Invalid migration")]
    InvalidMigration {},

    #[error("Unsupported migration path")]
    UnsupportedMigration {},

    #[error("Amount must be greater than zero")]
    ZeroAmount {},

    #[error("Invalid bonus multiplier")]
    InvalidMultiplier {},
}
EOF

# Create LP Locker state.rs
cat > contracts/lp-locker/src/state.rs << 'EOF'
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
EOF

echo "✓ LP Locker state.rs created"

# I'll continue with the rest of the files in the next command
echo "Continuing with message definitions..."

