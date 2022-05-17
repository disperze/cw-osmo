#![cfg(test)]

use crate::contract::instantiate;
use crate::ibc::{ibc_channel_connect, ibc_channel_open, ICS20_ORDERING, ICS20_VERSION};
use crate::state::ChannelInfo;

use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
    mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{attr, DepsMut, Event, IbcEndpoint, OwnedDeps};

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

pub fn mock_swap_events() -> Vec<Event> {
    return vec![
        Event::new("token_swapped").add_attributes(vec![
            attr("module", "gamm"),
            attr("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
            attr("pool_id", "497"),
            attr(
                "tokens_in",
                "10000000ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
            ),
            attr(
                "tokens_out",
                "338527564ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC",
            ),
            attr("module", "gamm"),
            attr("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
            attr("pool_id", "560"),
            attr("tokens_in", "36601070uosmo"),
            attr("tokens_out", "36601070uosmo"),
        ]),
        Event::new("transfer").add_attributes(vec![
            attr(
                "recipient",
                "osmo1h7yfu7x4qsv2urnkl4kzydgxegdfyjdry5ee4xzj98jwz0uh07rqdkmprr",
            ),
            attr("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
            attr(
                "amount",
                "10000000ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED",
            ),
            attr("recipient", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
            attr(
                "sender",
                "osmo10d8ddsydag5xrnl2kacmkjtdxddstvz4jvraqqpf6ss2n7fy6lkqw4sx2f",
            ),
            attr(
                "amount",
                "338527564ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC",
            ),
        ]),
    ];
}

pub fn mock_join_pool_events() -> Vec<Event> {
    return vec![
        Event::new("pool_joined").add_attributes(vec![
            attr("module", "gamm"),
            attr("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
            attr("pool_id", "1"),
            attr("tokens_in", "10000000uosmo"),
        ]),
        Event::new("coinbase").add_attributes(vec![
            attr("minter", "osmo1c9y7crgg6y9pfkq0y8mqzknqz84c3etr0kpcvj"),
            attr("amount", "74196993097318119147gamm/pool/1"),
        ]),
    ];
}

pub fn mock_exit_pool_events() -> Vec<Event> {
    return vec![
        Event::new("pool_exited").add_attributes(vec![
            attr("module", "gamm"),
            attr("sender", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
            attr("pool_id", "1"),
            attr("tokens_out", "9970022uosmo"),
        ]),
        Event::new("burn").add_attributes(vec![
            attr("burner", "osmo1c9y7crgg6y9pfkq0y8mqzknqz84c3etr0kpcvj"),
            attr("amount", "74196993097318119147gamm/pool/1"),
        ]),
    ];
}
