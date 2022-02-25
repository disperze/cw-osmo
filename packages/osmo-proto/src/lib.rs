pub mod proto_ext;
pub mod query;

/// The version (commit hash) of the Osmosis SDK used when generating this library.
pub const OSMOSIS_VERSION: &str = include_str!("types/OSMOSIS_COMMIT");

pub mod osmosis {
    pub mod gamm {
        pub mod v1beta1 {
            include!("types/osmosis.gamm.v1beta1.rs");
            include!("paths/osmosis.gamm.v1beta1.rs");
        }
    }
}
