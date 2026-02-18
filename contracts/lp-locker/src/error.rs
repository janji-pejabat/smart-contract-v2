use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] cosmwasm_std::OverflowError),

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

    #[error("Amount must be greater than zero")]
    ZeroAmount {},
}
