use cosmwasm_std::{Binary, Decimal, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This is the message we send over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PacketMsg {
    /// The unique identifier of this request, as specified by the client
    pub client_id: Option<String>,
    pub query: GammMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GammMsg {
    SpotPrice(SpotPriceMsg),
    EstimateSwap(EstimateSwapMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPriceMsg {
    pub pool: Uint64,
    pub token_in: String,
    pub token_out: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateSwapMsg {
    pub pool: Uint64,
    pub sender: String,
    pub amount: Uint128,
    pub token_in: String,
    pub token_out: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPriceAck {
    pub price: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateSwapAck {
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum PacketAck {
    Result(Binary),
    Error(String),
}
