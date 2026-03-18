use tonic::transport::Channel;
use zisk_distributed_grpc_api::zisk_coordinator_api_client::ZiskCoordinatorApiClient;

/// Thin wrapper around the generated gRPC client for the coordinator's
/// external API (`ZiskCoordinatorApi`).
///
/// Uses lazy connection so the node starts even when the coordinator is
/// temporarily unreachable.
#[derive(Clone)]
pub struct CoordinatorClient {
    pub(crate) inner: ZiskCoordinatorApiClient<Channel>,
}

impl CoordinatorClient {
    pub fn connect(url: String) -> Self {
        let channel = Channel::from_shared(url).expect("valid coordinator URL").connect_lazy();
        Self { inner: ZiskCoordinatorApiClient::new(channel) }
    }
}
