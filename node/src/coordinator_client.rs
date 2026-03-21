use std::ops::{Deref, DerefMut};

use tonic::transport::Channel;
use zisk_distributed_grpc_api::zisk_coordinator_api_client::ZiskCoordinatorApiClient;

/// Thin wrapper around the generated gRPC client for the coordinator's
/// external API (`ZiskCoordinatorApi`).
///
/// Uses lazy connection so the node starts even when the coordinator is
/// temporarily unreachable.
///
/// All methods from `ZiskCoordinatorApiClient` are available directly via
/// `Deref`/`DerefMut` — no forwarding methods needed.
#[derive(Clone)]
pub struct ZiskCoordinatorClient {
    inner: ZiskCoordinatorApiClient<Channel>,
}

impl ZiskCoordinatorClient {
    pub fn connect(url: String) -> anyhow::Result<Self> {
        let channel = Channel::from_shared(url)?.connect_lazy();
        Ok(Self { inner: ZiskCoordinatorApiClient::new(channel) })
    }
}

impl Deref for ZiskCoordinatorClient {
    type Target = ZiskCoordinatorApiClient<Channel>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ZiskCoordinatorClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
