use serde::{Deserialize, Serialize};

use cosmwasm_std::Timestamp;
use cw_storage_plus::Map;

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct ChannelData {
    pub last_update_time: Timestamp,
}

pub const CHANNELS_INFO: Map<&str, ChannelData> = Map::new("channels");
