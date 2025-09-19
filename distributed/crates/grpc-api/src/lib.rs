// Include the generated gRPC code
mod distributed_api_proto {
    tonic::include_proto!("distributed.api.v1");
}
pub mod conversions;

pub use distributed_api_proto::distributed_api_server;
pub use distributed_api_proto::*;
