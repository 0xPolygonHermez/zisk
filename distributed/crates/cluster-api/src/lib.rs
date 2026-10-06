//! Internal cluster API: the worker ↔ coordinator gRPC contract.
//!
//! The message and service types are generated from the `zisk.distributed.api.v1`
//! proto and re-exported at the crate root; [`conversions`] bridges them to the
//! domain types used inside the coordinator and worker.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

// Include the generated gRPC code
mod distributed_api_proto {
    // Generated code — the proto definitions are the source of truth.
    #![allow(missing_docs)]
    tonic::include_proto!("zisk.distributed.api.v1");
}

/// Proto ↔ domain conversions for the cluster API.
pub mod conversions;

pub use distributed_api_proto::zisk_distributed_api_server;
pub use distributed_api_proto::*;

/// Maximum gRPC message size for the cluster API (128 MB).
pub const MAX_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MB
