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

    #[error("Vesting not found")]
    VestingNotFound {},

    #[error("Vesting already revoked")]
    VestingAlreadyRevoked {},

    #[error("Vesting is not revocable")]
    NotRevocable {},

    #[error("Invalid schedule: {reason}")]
    InvalidSchedule { reason: String },

    #[error("Invalid milestone: {reason}")]
    InvalidMilestone { reason: String },

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Invalid input: {reason}")]
    InvalidInput { reason: String },
}
