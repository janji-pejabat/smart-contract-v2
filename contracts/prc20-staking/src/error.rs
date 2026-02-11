use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Contract is paused")]
    Paused {},

    #[error("Room not found")]
    RoomNotFound {},

    #[error("Invalid stake token")]
    InvalidStakeToken {},

    #[error("Insufficient stake amount")]
    InsufficientStake {},

    #[error("NFT requirement not met")]
    NFTRequirementNotMet {},

    #[error("Auto-compound not allowed for this user/room")]
    AutoCompoundNotAllowed {},

    #[error("Cooldown period not elapsed")]
    CooldownNotElapsed {},

    #[error("Invalid reward token")]
    InvalidRewardToken {},

    #[error("No rewards to claim")]
    NoRewards {},

    #[error("Invalid room configuration")]
    InvalidRoomConfig {},

    #[error("Overflow")]
    Overflow { #[from] source: cosmwasm_std::OverflowError },
}
