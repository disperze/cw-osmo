use cosmwasm_std::{Binary, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The format for sending an ics20 packet.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/applications/transfer/v1/transfer.proto#L11-L20
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Ics20Packet {
    /// amount of tokens to transfer is encoded as a string, but limited to u64 max
    pub amount: Uint128,
    /// the token denomination to be transferred
    pub denom: String,
    /// the recipient address on the destination chain
    pub receiver: String,
    /// the sender address
    pub sender: String,
    /// Action packet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<OsmoPacket>,
}

impl Ics20Packet {
    pub fn new<T: Into<String>>(amount: Uint128, denom: T, sender: &str, receiver: &str) -> Self {
        Ics20Packet {
            denom: denom.into(),
            amount,
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            action: None,
        }
    }
}

/// This is a generic ICS acknowledgement format.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ics20Ack {
    Result(Binary),
    Error(String),
}

pub struct Voucher {
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OsmoPacket {
    /// Swap assets.
    Swap(SwapPacket),
    /// Join a specific pool.
    JoinPool(JoinPoolPacket),
    /// Exit a specific pool.
    ExitPool(ExitPoolPacket),
    LockupAccount {},
    Lock(LockPacket),
    Claim(ClaimPacket),
    Unlock(UnlockPacket),
}

/// Swap Packet
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapPacket {
    pub routes: Vec<SwapAmountInRoute>,
    pub token_out_min_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapAmountInRoute {
    pub pool_id: Uint64,
    pub token_out_denom: String,
}

/// JoinPool Packet
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct JoinPoolPacket {
    pub pool_id: Uint64,
    pub share_out_min_amount: Uint128,
}

/// JoinPool Packet
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExitPoolPacket {
    pub token_out_denom: String,
    pub token_out_min_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockPacket {
    pub duration: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimPacket {
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UnlockPacket {
    pub id: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AmountResultAck {
    pub amount: Uint128,
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockResultAck {
    pub lock_id: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockupAck {
    pub contract: String,
}
