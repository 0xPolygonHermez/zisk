mod common_proto {
    tonic::include_proto!("zisk.common.v1");
}

mod coordinator_api_proto {
    #![allow(clippy::doc_lazy_continuation)]
    tonic::include_proto!("zisk.coordinator.v1");
}

mod cluster_api_proto {
    #![allow(clippy::doc_lazy_continuation)]
    tonic::include_proto!("zisk.cluster.v1");
}

pub mod conversions;

// ZiskCoordinatorApi — external API
pub use coordinator_api_proto::zisk_coordinator_api_client;
pub use coordinator_api_proto::zisk_coordinator_api_server;
pub use coordinator_api_proto::*;

// ZiskClusterApi — internal coordinator↔worker protocol
pub use cluster_api_proto::zisk_cluster_api_client;
pub use cluster_api_proto::zisk_cluster_api_server;
pub use cluster_api_proto::*;

// Shared types
pub use common_proto::*;

pub const MAX_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MB
