use crate::error::ContractError;
use cosmwasm_std::{
    attr, entry_point, DepsMut, Env, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
    IbcReceiveResponse, StdError, StdResult,
};
use osmo_bindings::OsmosisQuery;

use crate::relay::{ack_fail, enforce_order_and_version, on_recv_packet};
use crate::state::{ChannelData, CHANNELS_INFO};

#[entry_point]
pub fn ibc_channel_open(
    _deps: DepsMut<OsmosisQuery>,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<(), ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    Ok(())
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let channel = msg.channel();
    enforce_order_and_version(channel, msg.counterparty_version())?;

    let channel_id = &channel.endpoint.channel_id;
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
    deps: DepsMut<OsmosisQuery>,
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
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    on_recv_packet(deps, &msg.packet).or_else(|err| {
        Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attributes(vec![
                attr("action", "receive"),
                attr("error", err.to_string()),
            ]))
    })
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    Err(StdError::generic_err("cannot receive acknowledgement"))
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut<OsmosisQuery>,
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

    use crate::ibc_msg::{GammMsg, PacketMsg, SpotPriceMsg};
    use crate::relay::QUERY_VERSION;
    use crate::test_helpers::mock_dependencies;
    use cosmwasm_std::testing::{
        mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_ibc_packet_recv,
        mock_ibc_packet_timeout, mock_info, MockApi, MockStorage,
    };
    use cosmwasm_std::{from_slice, IbcAcknowledgement, IbcOrder, OwnedDeps};
    use osmo_bindings_test::OsmosisApp;

    const CREATOR: &str = "creator";


    fn setup() -> OwnedDeps<MockStorage, MockApi, OsmosisApp, OsmosisQuery> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // connect will run through the entire handshake to set up a proper connect and
    // save the account (tested in detail in `proper_handshake_flow`)
    fn connect(mut deps: DepsMut<OsmosisQuery>, channel_id: &str) {
        let handshake_open =
            mock_ibc_channel_open_init(channel_id, IbcOrder::Unordered, QUERY_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, IbcOrder::Unordered, QUERY_VERSION);
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

        let ack_msg =
            mock_ibc_packet_ack(channel_id, b"{}", IbcAcknowledgement::new(&[1])).unwrap();
        ibc_packet_ack(deps.as_mut(), mock_env(), ack_msg).unwrap_err();

        let timeout_msg = mock_ibc_packet_timeout(channel_id, b"{}").unwrap();
        ibc_packet_timeout(deps.as_mut(), mock_env(), timeout_msg).unwrap_err();
    }

    #[test]
    fn rcv_query_packet() {
        let mut deps = setup();
        let channel_id = "channel-1234";
        connect(deps.as_mut(), channel_id);

        let packet = PacketMsg {
            client_id: None,
            query: GammMsg::SpotPrice(SpotPriceMsg{
                pool: 1u8.into(),
                token_in: "uosmo".into(),
                token_out: "uatom".into(),
            })
        };
        let rcv_msg = mock_ibc_packet_recv(channel_id, &packet).unwrap();
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), rcv_msg).unwrap();

        let error = res.attributes.iter().find(|r| r.key == "error".to_string());
        // TODO: Unsupported query type: Stargate
        assert_eq!("error", error.unwrap().key);
        assert_eq!(0, res.messages.len());
    }
}
