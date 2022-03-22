use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This is the message we send over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PacketMsg {
    /// The unique identifier of this request, as specified by the client
    pub client_id: Option<String>,
    pub path: String,
    pub data: Binary,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum PacketAck {
    Result(Binary),
    Error(String),
}

/// This is the success response we send on ack for PacketMsg::SpotPrice.
/// Just acknowledge success or error
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPriceAck {
    pub price: String,
}
