pub mod proto {
    #![allow(clippy::large_enum_variant)]
    tonic::include_proto!("zisk.gateway.v1");
}

pub use proto::zisk_gateway_api_client::ZiskGatewayApiClient;
