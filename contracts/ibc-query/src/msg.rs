use cosmwasm_std::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::ChannelData;

/// This needs no info. Owner of the contract is whoever signed the InstantiateMsg.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Shows all open accounts (incl. remote info)
    ListChannels {},
    // Get account for one channel
    Channel { id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChannelInfo {
    pub channel_id: String,
    pub creation_time: Timestamp,
}

impl ChannelInfo {
    pub fn convert(channel_id: String, input: ChannelData) -> Self {
        ChannelInfo {
            channel_id,
            creation_time: input.creation_time,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChannelResponse {
    pub creation_time: Timestamp,
}

impl From<ChannelData> for ChannelResponse {
    fn from(input: ChannelData) -> Self {
        ChannelResponse {
            creation_time: input.creation_time,
        }
    }
}
