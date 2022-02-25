use crate::proto_ext::ProtoUrl;

impl ProtoUrl for QuerySpotPriceRequest {
    fn path(&self) -> String {
        String::from("/osmosis.gamm.v1beta1.Query/SpotPrice")
    }
}
