use std::time::Duration;

use anyhow::{Context, Result};
use tonic::transport::Channel;
use zisk_gateway_api::dto::{DomainJobKind, RegisterGuestProgramRequestDto};
use zisk_gateway_api::grpc::ZiskGatewayApiClient;

use crate::job::Job;

#[derive(Clone)]
pub struct GatewayClient {
    inner: ZiskGatewayApiClient<Channel>,
}

impl GatewayClient {
    pub fn connect(
        url: impl Into<String>,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self> {
        let channel = block_on(async {
            tonic::transport::Endpoint::from_shared(url.into())
                .context("Invalid gateway URL")?
                .connect_timeout(connect_timeout)
                .timeout(request_timeout)
                .connect()
                .await
                .context("Failed to connect to gateway")
        })?;
        Ok(Self { inner: ZiskGatewayApiClient::new(channel) })
    }

    pub fn register_program(&self, elf: Vec<u8>) -> Result<String> {
        block_on(async {
            let mut gw = self.inner.clone();
            let req = RegisterGuestProgramRequestDto { zisk_elf: elf };
            let resp =
                gw.register_guest_program(req).await.context("RegisterGuestProgram RPC failed")?;
            Ok(resp.into_inner().hash_id)
        })
    }

    pub fn submit_job(&self, kind: DomainJobKind) -> Result<Job> {
        let job_id = block_on(async {
            let mut gw = self.inner.clone();
            let resp = gw.job_request(kind).await.context("JobRequest RPC failed")?;
            Ok::<_, anyhow::Error>(resp.into_inner().job_id)
        })?;
        Job::new(job_id, self.clone())
    }

    pub fn async_client(&self) -> ZiskGatewayApiClient<Channel> {
        self.inner.clone()
    }
}

pub fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}
