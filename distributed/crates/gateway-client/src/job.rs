use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use uuid::Uuid;
use zisk_gateway_api::dto::{
    DomainJobEvent, DomainJobKindResponse, DomainJobStatus, TerminalStatus,
};
use zisk_gateway_api::grpc::proto::{CancelJobRequest, WaitJobResultRequest, WatchJobRequest};

use crate::client::GatewayClient;

const WAIT_POLL_SECS: u32 = 5;

pub struct WatchHandle {
    task: tokio::task::JoinHandle<()>,
}

impl WatchHandle {
    pub fn abort(&self) {
        self.task.abort();
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone)]
pub struct Job {
    job_id: Uuid,
    client: GatewayClient,
}

impl Job {
    pub(crate) fn new(job_id: String, client: GatewayClient) -> Result<Self> {
        let job_id = Uuid::parse_str(&job_id)
            .with_context(|| format!("gateway returned invalid job_id UUID: {job_id}"))?;
        Ok(Self { client, job_id })
    }

    pub fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// Spawn a background task that watches job events via the gateway stream.
    ///
    /// Terminal events (`Completed`, `Cancelled`, `Failed`) are NOT forwarded to `on_event`.
    /// Use [`Self::wait_async`] to receive the terminal result.
    pub fn spawn_watch(
        &self,
        on_event: impl Fn(DomainJobEvent) -> bool + Send + Sync + 'static,
    ) -> WatchHandle {
        let mut gw = self.client.async_client();
        let job_id = self.job_id.to_string();
        let task = tokio::spawn(async move {
            if let Ok(resp) = gw.watch_job(WatchJobRequest { job_id }).await {
                let mut stream = resp.into_inner();
                while let Some(Ok(proto_event)) = stream.next().await {
                    match DomainJobEvent::try_from(proto_event) {
                        Ok(domain_event) => {
                            if on_event(domain_event) {
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        });
        WatchHandle { task }
    }

    pub fn wait(&self, timeout: Option<Duration>) -> Result<TerminalStatus> {
        crate::client::block_on(self.wait_async(timeout))
    }

    pub async fn wait_async(&self, timeout: Option<Duration>) -> Result<TerminalStatus> {
        let deadline = timeout.map(|d| tokio::time::Instant::now() + d);
        let mut gw = self.client.async_client();

        loop {
            if let Some(dl) = deadline {
                if tokio::time::Instant::now() >= dl {
                    anyhow::bail!("job timed out after {:?}", timeout.unwrap());
                }
            }

            let proto_resp = gw
                .wait_job_result(WaitJobResultRequest {
                    job_id: self.job_id.to_string(),
                    timeout_seconds: Some(WAIT_POLL_SECS),
                })
                .await
                .map_err(|e| anyhow::anyhow!("WaitJobResult failed: {e}"))?
                .into_inner();

            let job_status: DomainJobStatus = proto_resp
                .job_status
                .ok_or_else(|| anyhow::anyhow!("job_status must be set"))?
                .try_into()
                .map_err(|e: String| anyhow::anyhow!("invalid job_status: {e}"))?;

            let terminal = match job_status {
                DomainJobStatus::Completed => {
                    let kind: DomainJobKindResponse = proto_resp
                        .result
                        .ok_or_else(|| anyhow::anyhow!("completed job missing result"))?
                        .try_into()
                        .map_err(|e: String| anyhow::anyhow!("invalid job result: {e}"))?;
                    TerminalStatus::Completed(kind)
                }
                DomainJobStatus::Failed(f) => TerminalStatus::Failed(f),
                DomainJobStatus::Cancelled => TerminalStatus::Cancelled,
                _ => continue,
            };

            return Ok(terminal);
        }
    }

    pub fn cancel(&self) -> Result<bool> {
        crate::client::block_on(self.cancel_async())
    }

    pub async fn cancel_async(&self) -> Result<bool> {
        let mut gw = self.client.async_client();
        let resp = gw
            .cancel_job(CancelJobRequest { job_id: self.job_id.to_string() })
            .await
            .map_err(|e| anyhow::anyhow!("CancelJob RPC failed: {e}"))?;
        Ok(resp.into_inner().cancelled)
    }
}
