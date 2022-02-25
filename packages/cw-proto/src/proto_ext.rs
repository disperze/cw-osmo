use cosmwasm_std::{
    to_vec, Binary, ContractResult, CosmosMsg, Deps, Empty, QueryRequest, StdError, StdResult,
    SystemResult,
};

pub trait MessageExt: prost::Message {
    /// Serialize this protobuf message as a byte vector.
    fn to_bytes(&self) -> StdResult<Vec<u8>>;

    fn to_query(&self) -> StdResult<QueryRequest<Empty>>;

    fn to_msg(&self) -> StdResult<CosmosMsg>;
}

pub trait ProtoUrl {
    fn path(&self) -> String;
}

impl<M> MessageExt for M
where
    M: prost::Message + ProtoUrl,
{
    fn to_bytes(&self) -> StdResult<Vec<u8>> {
        let mut bytes = Vec::new();
        prost::Message::encode(self, &mut bytes)
            .map_err(|_| StdError::generic_err("cannot encode proto"))?;

        Ok(bytes)
    }

    fn to_query(&self) -> StdResult<QueryRequest<Empty>> {
        let data = self.to_bytes()?;
        let request: QueryRequest<Empty> = QueryRequest::Stargate {
            path: self.path(),
            data: data.into(),
        };

        Ok(request)
    }

    fn to_msg(&self) -> StdResult<CosmosMsg> {
        let data = self.to_bytes()?;

        let msg = CosmosMsg::Stargate {
            type_url: self.path(),
            value: data.into(),
        }
        .into();

        Ok(msg)
    }
}

pub fn query_proto<M: prost::Message + ProtoUrl, R: prost::Message + std::default::Default>(
    deps: Deps,
    msg: M,
) -> StdResult<R> {
    let request = msg.to_query()?;
    let result = query_raw(deps, request)?;

    let output = prost::Message::decode(result.as_slice())
        .map_err(|_| StdError::generic_err("cannot decode proto"))?;
    Ok(output)
}

pub fn query_raw(deps: Deps, request: QueryRequest<Empty>) -> StdResult<Binary> {
    let raw = to_vec(&request).map_err(|serialize_err| {
        StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
    })?;

    match deps.querier.raw_query(&raw) {
        SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
            "Querier system error: {}",
            system_err
        ))),
        SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            contract_err
        ))),
        SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
    }
}
