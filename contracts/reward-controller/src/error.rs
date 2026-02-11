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

    #[error("Pool not found")]
    PoolNotFound {},

    #[error("Stake not found")]
    StakeNotFound {},

    #[error("Locker not found or not locked")]
    InvalidLocker {},

    #[error("Claim interval not passed yet")]
    ClaimTooSoon {},

    #[error("No rewards to claim")]
    NoRewards {},

    #[error("Invalid emission rate")]
    InvalidEmissionRate {},

    #[error("Pool is disabled")]
    PoolDisabled {},

    #[error("Insufficient reward balance")]
    InsufficientRewards {},

    #[error("Overflow")]
    Overflow { #[from] source: cosmwasm_std::OverflowError },
}
