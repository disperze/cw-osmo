pub mod proto_ext;
pub mod query;
pub use prost::Message;
pub use prost_types::Duration;

/// The version (commit hash) of the Osmosis SDK used when generating this library.
pub const OSMOSIS_VERSION: &str = include_str!("types/OSMOSIS_COMMIT");

pub mod cosmos {
    /// Base functionality.
    pub mod base {
        /// Query support.
        pub mod query {
            pub mod v1beta1 {
                include!("types/cosmos.base.query.v1beta1.rs");
            }
        }

        pub mod v1beta1 {
            include!("types/cosmos.base.v1beta1.rs");
        }
    }
}

pub mod osmosis {
    pub mod gamm {
        pub mod v1beta1 {
            include!("types/osmosis.gamm.v1beta1.rs");
            include!("paths/osmosis.gamm.v1beta1.rs");
        }
    }

    pub mod lockup {
        include!("types/osmosis.lockup.rs");
        include!("paths/osmosis.lockup.rs");
    }
}
