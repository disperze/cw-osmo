use crate::error::ContractError;
use crate::ibc_msg::{EstimateSwapAck, GammMsg, PacketAck, PacketMsg, SpotPriceAck};
use cosmwasm_std::{
    from_slice, to_binary, Binary, DepsMut, IbcChannel, IbcOrder, IbcPacket, IbcReceiveResponse,
    QueryRequest,
};
use osmo_bindings::{EstimatePriceResponse, OsmosisQuery, SpotPriceResponse, SwapAmount};

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
    deps: DepsMut<OsmosisQuery>,
    packet: &IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let msg: PacketMsg = from_slice(&packet.data)?;

    let ack_data =
        match msg.query {
            GammMsg::SpotPrice(m) => {
                let re: QueryRequest<OsmosisQuery> = QueryRequest::Custom(
                    OsmosisQuery::spot_price(m.pool.u64(), &m.token_in, &m.token_out),
                );
                let res: SpotPriceResponse = deps.querier.query(&re)?;
                let ack = SpotPriceAck { price: res.price };
                to_binary(&ack)?
            }
            GammMsg::EstimateSwap(m) => {
                let re: QueryRequest<OsmosisQuery> =
                    QueryRequest::Custom(OsmosisQuery::estimate_price(
                        m.sender,
                        m.pool.u64(),
                        m.token_in,
                        m.token_out,
                        SwapAmount::In(m.amount),
                    ));
                let res: EstimatePriceResponse = deps.querier.query(&re)?;
                let ack = EstimateSwapAck {
                    amount: res.amount.as_in(),
                };

                to_binary(&ack)?
            }
        };

    Ok(IbcReceiveResponse::new()
        .set_ack(ack_success(ack_data))
        .add_attribute("action", "receive"))
}
