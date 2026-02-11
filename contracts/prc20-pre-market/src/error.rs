use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Market is paused")]
    Paused {},

    #[error("Listing not found")]
    ListingNotFound {},

    #[error("Listing is not active")]
    ListingNotActive {},

    #[error("Invalid price")]
    InvalidPrice {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Insufficient remaining tokens in listing")]
    InsufficientRemaining {},

    #[error("Buy amount below minimum limit")]
    BelowMinLimit {},

    #[error("Buy amount above maximum limit")]
    AboveMaxLimit {},

    #[error("Token is blacklisted")]
    TokenBlacklisted {},

    #[error("Not on whitelist")]
    NotOnWhitelist {},

    #[error("Self buying is not allowed")]
    SelfBuyNotAllowed {},
}
