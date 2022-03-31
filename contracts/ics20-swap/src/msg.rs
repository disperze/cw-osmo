use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::amount::Amount;
use crate::state::ChannelInfo;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct InitMsg {
    /// Default timeout for ics20 packets, specified in seconds
    pub default_timeout: u64,
    /// Lockup code ID
    pub lockup_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// This allows us to transfer *exactly one* native token
    Transfer(TransferMsg),
}

/// This is the message we accept via Receive
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TransferMsg {
    /// The local channel to send the packets on
    pub channel: String,
    /// The remote address to send to.
    /// Don't use HumanAddress as this will likely have a different Bech32 prefix than we use
    /// and cannot be validated locally
    pub remote_address: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Show all channels we have connected to. Return type is ListChannelsResponse.
    ListChannels {},
    /// Returns the details of the name channel, error if not created.
    /// Return type: ChannelResponse.
    Channel { id: String },
    /// Show the Config. Returns ConfigResponse
    Config {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ChannelResponse {
    /// Information on the channel's connection
    pub info: ChannelInfo,
    /// How many tokens we currently have pending over this channel
    pub balances: Vec<Amount>,
    /// The total number of tokens that have been sent over this channel
    /// (even if many have been returned, so balance is low)
    pub total_sent: Vec<Amount>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ConfigResponse {
    pub default_timeout: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct LockupInitMsg {
    pub admin: String,
}
