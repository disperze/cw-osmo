use crate::error::ContractError;
use crate::ibc_msg::{PacketAck, PacketMsg};
use cosmwasm_std::{
    from_slice, to_binary, Binary, DepsMut, Empty, IbcChannel, IbcOrder, IbcPacket,
    IbcReceiveResponse, QueryRequest,
};
use cw_osmo_proto::query::query_raw;

pub const QUERY_VERSION: &str = "cw-query-1";
pub const QUERY_ORDERING: IbcOrder = IbcOrder::Unordered;

pub fn ack_success(result: Binary) -> Binary {
    let res = PacketAck::Result(result);
    to_binary(&res).unwrap()
}

pub fn ack_fail(err: String) -> Binary {
    let res = PacketAck::Error(err);
    to_binary(&res).unwrap()
}

pub fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version.as_str() != QUERY_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            default_version: QUERY_VERSION.to_string(),
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != QUERY_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                default_version: QUERY_VERSION.to_string(),
                version: version.to_string(),
            });
        }
    }
    if channel.order != QUERY_ORDERING {
        return Err(ContractError::OnlyUnorderedChannel {});
    }
    Ok(())
}

pub fn on_recv_packet(
    deps: DepsMut,
    packet: &IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let msg: PacketMsg = from_slice(&packet.data)?;
    assert_allowed_path(msg.path.as_str())?;

    let request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: msg.path,
        data: msg.data,
    };

    let result = query_raw(deps.as_ref(), request)?;

    Ok(IbcReceiveResponse::new()
        .set_ack(ack_success(result))
        .add_attribute("action", "receive"))
}

fn assert_allowed_path(path: &str) -> Result<(), ContractError> {
    let deny_paths = vec!["/cosmos.tx.", "/cosmos.base.tendermint."];
    for deny_path in deny_paths {
        if path.starts_with(deny_path) {
            return Err(ContractError::InvalidQueryPath {});
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_query_paths() {
        let invalid_path = "/cosmos.tx.v1beta1.Service/GetTx";
        let valid_path = "/osmosis.gamm.v1beta1.Query/SpotPrice";

        assert_allowed_path(invalid_path).unwrap_err();

        assert_allowed_path(valid_path).unwrap()
    }
}
