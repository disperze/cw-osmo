use cosmwasm_std::{Binary, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This is the message we send over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PacketMsg {
    /// gamm/SpotPrice
    SpotPrice(SpotPricePacket),
    /// gamm/EstimateSwapExactAmountIn
    EstimateSwapAmountIn(EstimateSwapAmountInPacket),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPricePacket {
    pub pool_id: Uint64,
    pub token_in: String,
    pub token_out: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateSwapAmountInPacket {
    pub sender: String,
    pub pool_id: Uint64,
    pub token_in: String,
    pub routes: Vec<SwapAmountInRoute>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapAmountInRoute {
    pub pool_id: Uint64,
    pub token_out_denom: String,
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

impl From<SwapAmountInRoute> for cw_osmo_proto::osmosis::gamm::v1beta1::SwapAmountInRoute  {
    fn from(msg: SwapAmountInRoute) -> cw_osmo_proto::osmosis::gamm::v1beta1::SwapAmountInRoute {
        cw_osmo_proto::osmosis::gamm::v1beta1::SwapAmountInRoute {
            pool_id: msg.pool_id.into(),
            token_out_denom: msg.token_out_denom,
        }
    }
}
