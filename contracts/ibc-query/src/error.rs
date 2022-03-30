use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Only supports channel with ibc version {default_version}, got {version}")]
    InvalidIbcVersion {
        default_version: String,
        version: String,
    },

    #[error("Only supports unordered channel")]
    OnlyUnorderedChannel {},

    #[error("Query path is not allowed")]
    InvalidQueryPath {},
}
