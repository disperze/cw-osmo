use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, Order, QueryResponse, Response,
    StdResult,
};
use osmo_bindings::OsmosisQuery;

use crate::msg::{ChannelInfo, ChannelResponse, InstantiateMsg, ListChannelsResponse, QueryMsg};
use crate::state::CHANNELS_INFO;

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Channel { id } => to_binary(&query_channel(deps, id)?),
        QueryMsg::ListChannels {} => to_binary(&query_list_channels(deps)?),
    }
}

fn query_channel(deps: Deps<OsmosisQuery>, channel_id: String) -> StdResult<ChannelResponse> {
    let channel = CHANNELS_INFO.load(deps.storage, &channel_id)?;
    Ok(channel.into())
}

fn query_list_channels(deps: Deps<OsmosisQuery>) -> StdResult<ListChannelsResponse> {
    let channels: StdResult<Vec<_>> = CHANNELS_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            let (k, account) = r?;
            Ok(ChannelInfo::convert(k, account))
        })
        .collect();
    Ok(ListChannelsResponse {
        channels: channels?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::mock_dependencies;
    use cosmwasm_std::testing::{mock_env, mock_info};

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
