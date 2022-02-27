use crate::proto_ext::ProtoUrl;

impl ProtoUrl for QuerySpotPriceRequest {
    fn path(&self) -> &str {
        "/osmosis.gamm.v1beta1.Query/SpotPrice"
    }
}

impl ProtoUrl for QuerySwapExactAmountInRequest {
    fn path(&self) -> &str {
        "/osmosis.gamm.v1beta1.Query/EstimateSwapExactAmountIn"
    }
}

impl ProtoUrl for MsgSwapExactAmountIn {
    fn path(&self) -> &str {
        "/osmosis.gamm.v1beta1.MsgSwapExactAmountIn"
    }
}
