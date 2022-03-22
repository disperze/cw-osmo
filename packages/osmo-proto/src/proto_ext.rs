use cosmwasm_std::{CosmosMsg, Empty, QueryRequest, StdError, StdResult};

pub trait MessageExt: prost::Message {
    /// Serialize this protobuf message as a byte vector.
    fn to_bytes(&self) -> StdResult<Vec<u8>>;

    fn to_query(&self) -> StdResult<QueryRequest<Empty>>;

    fn to_msg(&self) -> StdResult<CosmosMsg>;
}

pub trait ProtoUrl {
    fn path(&self) -> &str;
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
            path: self.path().to_string(),
            data: data.into(),
        };

        Ok(request)
    }

    fn to_msg(&self) -> StdResult<CosmosMsg> {
        let data = self.to_bytes()?;

        let msg = CosmosMsg::Stargate {
            type_url: self.path().to_string(),
            value: data.into(),
        };

        Ok(msg)
    }
}

pub fn proto_decode<M: prost::Message + std::default::Default>(data: &[u8]) -> StdResult<M> {
    prost::Message::decode(data).map_err(|_| StdError::generic_err("cannot decode proto"))
}
