use crate::parse::{
    find_attributes, find_event_type, parse_coin, OSMOSIS_ATTRIBUTE_TOKEN_OUT, OSMOSIS_EVENT_SWAP,
};
use crate::ContractError;
use cosmwasm_std::{Binary, Event, Uint128, Uint64};
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

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.amount.u128() > (u64::MAX as u128) {
            Err(ContractError::AmountOverflow {})
        } else {
            Ok(())
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
    /// denom is from source chain.
    pub our_chain: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OsmoPacket {
    Swap(SwapPacket),
}

/// Swap Packet
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapPacket {
    pub sender: String,
    pub routes: Vec<SwapAmountInRoute>,
    pub token_out_min_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapAmountInRoute {
    pub pool_id: Uint64,
    pub token_out_denom: String,
}

/// This is the success response we send on ack for PacketMsg::Balance.
/// Just acknowledge success or error
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapAmountInAck {
    pub amount: Uint128,
    pub denom: String,
}

pub fn parse_swap_out(events: Vec<Event>) -> Result<SwapAmountInAck, ContractError> {
    let event = find_event_type(events, OSMOSIS_EVENT_SWAP);
    if event.is_none() {
        return Err(ContractError::SwapOutputNotFound {});
    }

    let values = find_attributes(event.unwrap().attributes, OSMOSIS_ATTRIBUTE_TOKEN_OUT);
    if values.is_empty() {
        return Err(ContractError::SwapOutputNotFound {});
    }

    let token_out_str = values.last().unwrap();
    let token_out = parse_coin(token_out_str.as_str())?;

    let swap_ack = SwapAmountInAck {
        amount: token_out.amount,
        denom: token_out.denom,
    };

    Ok(swap_ack)
}
