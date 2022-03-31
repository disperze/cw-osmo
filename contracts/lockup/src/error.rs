use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Empty balance")]
    NoBalance {},

    #[error("Invalid lock id")]
    InvalidLockId {},

    #[error("Invalid token denomination")]
    InvalidEmptyDenom {},

    #[error("Only accept one token")]
    NoOneToken {},

    #[error("Missing reply data")]
    NoReplyData {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },
}
