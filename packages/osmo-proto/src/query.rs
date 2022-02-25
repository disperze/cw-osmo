use crate::proto_ext::{MessageExt, ProtoUrl};
use cosmwasm_std::{
    to_vec, Binary, ContractResult, Deps, Empty, QueryRequest, StdError, StdResult, SystemResult,
};

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
