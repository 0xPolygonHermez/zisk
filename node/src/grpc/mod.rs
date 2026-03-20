mod user_api_proto {
    #![allow(clippy::doc_lazy_continuation)]
    tonic::include_proto!("zisk.user.v1");
}

pub mod conversions;
pub mod logging;
pub mod user_api;

// User-facing API (ZiskUserApi)
pub mod user {
    pub use super::user_api_proto::zisk_user_api_server;
    pub use super::user_api_proto::*;
}

pub const MAX_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MiB
