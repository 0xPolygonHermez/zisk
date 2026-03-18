use crate::grpc::zisk_node_api_server::ZiskNodeApi;
use crate::grpc::{GetNodeInfoRequest, NodeInfo};
use tonic::{Request, Response, Status};

pub struct NodeApiService;

impl NodeApiService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NodeApiService {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl ZiskNodeApi for NodeApiService {
    async fn get_node_info(
        &self,
        _request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<NodeInfo>, Status> {
        Ok(Response::new(NodeInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: "ready".to_string(),
        }))
    }
}
