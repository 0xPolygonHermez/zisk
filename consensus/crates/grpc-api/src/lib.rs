// Include the generated gRPC code
mod consensus_api_proto {
    tonic::include_proto!("consensus.api.v1");
}

// Conversions between common types and gRPC types
pub mod conversions;

// Re-export all the generated types
pub use consensus_api_proto::*;

// Make the server types easily accessible
pub use consensus_api_proto::consensus_api_server;
