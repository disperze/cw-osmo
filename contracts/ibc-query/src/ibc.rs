use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, DepsMut, Empty, Env, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, QueryRequest, StdError,
    StdResult,
};
use cw_osmo_proto::query::query_raw;

use crate::ibc_msg::{PacketAck, PacketMsg};
use crate::state::{ChannelData, CHANNELS_INFO};

pub const QUERY_VERSION: &str = "cw-query-1";
pub const QUERY_ORDERING: IbcOrder = IbcOrder::Unordered;

fn ack_success(result: Binary) -> Binary {
    let res = PacketAck::Result(result);
    to_binary(&res).unwrap()
}

fn ack_fail(err: String) -> Binary {
    let res = PacketAck::Error(err);
    to_binary(&res).unwrap()
}

#[entry_point]
pub fn ibc_channel_open(_deps: DepsMut, _env: Env, msg: IbcChannelOpenMsg) -> StdResult<()> {
    let channel = msg.channel();

    if channel.order != QUERY_ORDERING {
        return Err(StdError::generic_err("Only supports unordered channels"));
    }

    if channel.version.as_str() != QUERY_VERSION {
        return Err(StdError::generic_err(format!(
            "Must set version to `{}`",
            QUERY_VERSION
        )));
    }

    if let Some(version) = msg.counterparty_version() {
        if version != QUERY_VERSION {
            return Err(StdError::generic_err(format!(
                "Counterparty version must be `{}`",
                QUERY_VERSION
            )));
        }
    }

    Ok(())
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();

    let channel_id = &channel.endpoint.channel_id;

    // create an account holder the channel exists (not found if not registered)
    let data = ChannelData {
        creation_time: env.block.time,
    };
    CHANNELS_INFO.save(deps.storage, channel_id, &data)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();

    // remove the channel
    let channel_id = &channel.endpoint.channel_id;
    CHANNELS_INFO.remove(deps.storage, channel_id);

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    let packet: PacketMsg = from_slice(&msg.packet.data)?;
    let allow_res = assert_allowed_path(packet.path.as_str());
    if let Err(err) = allow_res {
        return Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attribute("action", "receive")
            .add_attribute("error", err.to_string()));
    }

    let request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: packet.path,
        data: packet.data,
    };

    let result = query_raw(deps.as_ref(), request);

    match result {
        Ok(data) => Ok(IbcReceiveResponse::new()
            .set_ack(ack_success(data))
            .add_attribute("action", "receive")),
        Err(err) => Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attribute("action", "receive")
            .add_attribute("error", err.to_string())),
    }
}

pub fn assert_allowed_path(path: &str) -> StdResult<()> {
    let deny_paths = vec!["/cosmos.tx.", "/cosmos.base.tendermint."];
    for deny_path in deny_paths {
        if path.starts_with(deny_path) {
            return Err(StdError::generic_err(
                "path is not allowed from the contract",
            ));
        }
    }

    Ok(())
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    Err(StdError::generic_err("cannot receive acknowledgement"))
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Err(StdError::generic_err("cannot cause a packet timeout"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{instantiate, query};
    use crate::msg::{ChannelResponse, InstantiateMsg, QueryMsg};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init, mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_ibc_packet_timeout, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{IbcAcknowledgement, IbcOrder, OwnedDeps};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // connect will run through the entire handshake to set up a proper connect and
    // save the account (tested in detail in `proper_handshake_flow`)
    fn connect(mut deps: DepsMut, channel_id: &str) {
        let handshake_open =
            mock_ibc_channel_open_init(channel_id, IbcOrder::Unordered, QUERY_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, IbcOrder::Ordered, QUERY_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();

        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", IbcOrder::Ordered, QUERY_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", IbcOrder::Unordered, "reflect");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake =
            mock_ibc_channel_open_try("channel-12", IbcOrder::Unordered, QUERY_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }

    #[test]
    fn proper_handshake_flow() {
        // setup and connect handshake
        let mut deps = setup();
        let channel_id = "channel-1234";
        connect(deps.as_mut(), channel_id);

        // check for empty account
        let q = QueryMsg::Channel {
            id: channel_id.into(),
        };
        let r = query(deps.as_ref(), mock_env(), q).unwrap();
        let acct: ChannelResponse = from_slice(&r).unwrap();
        assert_eq!(true, acct.creation_time.nanos() > 0);

        // account should be set up
        let q = QueryMsg::Channel {
            id: channel_id.into(),
        };
        let r = query(deps.as_ref(), mock_env(), q).unwrap();
        let acct: ChannelResponse = from_slice(&r).unwrap();
        assert_eq!(true, acct.creation_time.nanos() > 0);
    }

    #[test]
    fn no_ack_packet_allowed() {
        let mut deps = setup();
        let channel_id = "channel-1234";
        connect(deps.as_mut(), channel_id);

        let ack_msg = mock_ibc_packet_ack(channel_id, b"{}", IbcAcknowledgement::new(&[1])).unwrap();
        ibc_packet_ack(deps.as_mut(), mock_env(), ack_msg).unwrap_err();

        let timeout_msg = mock_ibc_packet_timeout(channel_id, b"{}").unwrap();
        ibc_packet_timeout(deps.as_mut(), mock_env(), timeout_msg).unwrap_err();
    }
}
