#![cfg(test)]

use crate::contract::instantiate;
use crate::ibc::{ibc_channel_connect, ibc_channel_open, ICS20_ORDERING, ICS20_VERSION};
use crate::state::ChannelInfo;

use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
    mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{Attribute, DepsMut, Event, IbcEndpoint, OwnedDeps};

use crate::msg::InitMsg;

pub const DEFAULT_TIMEOUT: u64 = 3600; // 1 hour,
pub const CONTRACT_PORT: &str = "ibc:wasm1234567890abcdef";
pub const REMOTE_PORT: &str = "transfer";
pub const CONNECTION_ID: &str = "connection-2";

pub fn mock_channel_info(channel_id: &str) -> ChannelInfo {
    ChannelInfo {
        id: channel_id.to_string(),
        counterparty_endpoint: IbcEndpoint {
            port_id: "their_port".to_string(),
            channel_id: "channel-7".to_string(),
        },
        connection_id: CONNECTION_ID.into(),
    }
}

// we simulate instantiate and ack here
pub fn add_channel(mut deps: DepsMut, channel_id: &str) {
    let open_msg = mock_ibc_channel_open_init(channel_id, ICS20_ORDERING, ICS20_VERSION);
    ibc_channel_open(deps.branch(), mock_env(), open_msg).unwrap();
    let connect_msg = mock_ibc_channel_connect_ack(channel_id, ICS20_ORDERING, ICS20_VERSION);
    ibc_channel_connect(deps.branch(), mock_env(), connect_msg).unwrap();
}

pub fn setup(channels: &[&str]) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();

    // instantiate an empty contract
    let instantiate_msg = InitMsg {
        default_timeout: DEFAULT_TIMEOUT,
        lockup_id: 1,
    };
    let info = mock_info(&String::from("anyone"), &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    for channel in channels {
        add_channel(deps.as_mut(), channel);
    }
    deps
}

pub fn json_to_reply_proto(json: &str) -> Vec<u8> {
    let mut proto_data = vec![10u8, json.len() as u8];
    proto_data.extend_from_slice(json.as_bytes());

    proto_data
}

pub fn swap_events_mock() -> Vec<Event> {
    let mut ev1 = Event::new("token_swapped");
    ev1.attributes = vec![
        Attribute::new("module", "gamm"),
        Attribute::new("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
        Attribute::new("pool_id", "497"),
        Attribute::new(
            "tokens_in",
            "10000000ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        ),
        Attribute::new(
            "tokens_out",
            "338527564ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC",
        ),
        Attribute::new("module", "gamm"),
        Attribute::new("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
        Attribute::new("pool_id", "560"),
        Attribute::new("tokens_in", "36601070uosmo"),
        Attribute::new("tokens_out", "36601070uosmo"),
    ];
    let mut ev2 = Event::new("transfer");
    ev2.attributes = vec![
        Attribute::new(
            "recipient",
            "osmo1h7yfu7x4qsv2urnkl4kzydgxegdfyjdry5ee4xzj98jwz0uh07rqdkmprr",
        ),
        Attribute::new("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
        Attribute::new(
            "amount",
            "10000000ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
        ),
        Attribute::new("recipient", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
        Attribute::new(
            "sender",
            "osmo10d8ddsydag5xrnl2kacmkjtdxddstvz4jvraqqpf6ss2n7fy6lkqw4sx2f",
        ),
        Attribute::new(
            "amount",
            "338527564ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC",
        ),
    ];

    return vec![ev1, ev2];
}

pub fn join_pool_events_mock() -> Vec<Event> {
    let mut ev1 = Event::new("pool_joined");
    ev1.attributes = vec![
        Attribute::new("module", "gamm"),
        Attribute::new("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
        Attribute::new("pool_id", "1"),
        Attribute::new("tokens_in", "10000000uosmo"),
    ];
    let mut ev2 = Event::new("coinbase");
    ev2.attributes = vec![
        Attribute::new("minter", "osmo1c9y7crgg6y9pfkq0y8mqzknqz84c3etr0kpcvj"),
        Attribute::new("amount", "74196993097318119147gamm/pool/1"),
    ];

    return vec![ev1, ev2];
}

pub fn exit_pool_events_mock() -> Vec<Event> {
    let mut ev1 = Event::new("pool_exited");
    ev1.attributes = vec![
        Attribute::new("module", "gamm"),
        Attribute::new("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
        Attribute::new("pool_id", "1"),
        Attribute::new("tokens_out", "9970022uosmo"),
    ];
    let mut ev2 = Event::new("burn");
    ev2.attributes = vec![
        Attribute::new("burner", "osmo1c9y7crgg6y9pfkq0y8mqzknqz84c3etr0kpcvj"),
        Attribute::new("amount", "74196993097318119147gamm/pool/1"),
    ];

    return vec![ev1, ev2];
}
