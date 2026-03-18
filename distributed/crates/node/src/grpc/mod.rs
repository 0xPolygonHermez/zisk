mod node_api_proto {
    tonic::include_proto!("zisk.node.v1");
}

mod user_api_proto {
    #![allow(clippy::doc_lazy_continuation)]
    tonic::include_proto!("zisk.user.v1");
}

pub mod logging;
pub mod node_api;
pub mod user_api;

// Operator API (ZiskNodeApi)
pub use node_api_proto::zisk_node_api_server;
pub use node_api_proto::*;

// User-facing API (ZiskUserApi)
pub mod user {
    pub use super::user_api_proto::zisk_user_api_server;
    pub use super::user_api_proto::*;
}

pub const MAX_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MiB
