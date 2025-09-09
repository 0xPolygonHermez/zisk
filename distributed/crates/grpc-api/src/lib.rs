// Include the generated gRPC code
mod distributed_api_proto {
    tonic::include_proto!("distributed.api.v1");
}

// Conversions between common types and gRPC types
pub mod conversions;

// Re-export all the generated types
pub use distributed_api_proto::*;

// Make the server types easily accessible
pub use distributed_api_proto::distributed_api_server;
