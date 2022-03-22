use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, DepsMut, Empty, Env, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, QueryRequest, StdError,
    StdResult,
};
use cw_osmo_proto::query::query_raw;

use crate::ibc_msg::{PacketAck, PacketMsg};
use crate::state::{ChannelData, CHANNELS_INFO};

pub const GAMM_VERSION: &str = "cw-query-1";
pub const GAMM_ORDERING: IbcOrder = IbcOrder::Unordered;

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

    if channel.order != GAMM_ORDERING {
        return Err(StdError::generic_err("Only supports unordered channels"));
    }

    if channel.version.as_str() != GAMM_VERSION {
        return Err(StdError::generic_err(format!(
            "Must set version to `{}`",
            GAMM_VERSION
        )));
    }

    if let Some(version) = msg.counterparty_version() {
        if version != GAMM_VERSION {
            return Err(StdError::generic_err(format!(
                "Counterparty version must be `{}`",
                GAMM_VERSION
            )));
        }
    }

    Ok(())
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();

    let channel_id = &channel.endpoint.channel_id;

    // create an account holder the channel exists (not found if not registered)
    let data = ChannelData::default();
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

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{IbcOrder, OwnedDeps};

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
            mock_ibc_channel_open_init(channel_id, IbcOrder::Unordered, GAMM_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, IbcOrder::Ordered, GAMM_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();

        // this should send a WhoAmI request, which is received some blocks later
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", IbcOrder::Ordered, GAMM_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", IbcOrder::Unordered, "reflect");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake =
            mock_ibc_channel_open_try("channel-12", IbcOrder::Unordered, GAMM_VERSION);
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
        assert_eq!(0, acct.last_update_time.nanos());

        // account should be set up
        let q = QueryMsg::Channel {
            id: channel_id.into(),
        };
        let r = query(deps.as_ref(), mock_env(), q).unwrap();
        let acct: ChannelResponse = from_slice(&r).unwrap();
        assert_eq!(0, acct.last_update_time.nanos());
    }
}
