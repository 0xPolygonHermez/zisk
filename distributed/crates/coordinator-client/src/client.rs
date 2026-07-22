use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use tonic::transport::Channel;
use uuid::Uuid;
use zisk_coordinator_api::dto::{
    DomainAggregationProgramSpec, DomainJobKind, RegisterAggregationProgramRequestDto,
    RegisterGuestProgramRequestDto,
};
use zisk_coordinator_api::grpc::proto::{CancelJobRequest, JobKindExt, JobRequestExtMessage};
use zisk_coordinator_api::grpc::{ZiskCoordinatorApiClient, ZiskCoordinatorApiExtClient};

use crate::input_sender::InputSender;
use crate::job::Job;

/// Synchronous client for the ZisK coordinator gRPC API.
///
/// Wraps the async tonic client and exposes blocking methods (register
/// programs, submit/cancel jobs, open input/hints streams). Cheap to
/// [`Clone`] — clones share the same underlying channel.
#[derive(Clone)]
pub struct CoordinatorClient {
    inner: ZiskCoordinatorApiClient<Channel>,
    ext: ZiskCoordinatorApiExtClient<Channel>,
}

impl CoordinatorClient {
    /// Connect to the coordinator at `url` with the given connect and
    /// per-request timeouts.
    pub fn connect(
        url: impl Into<String>,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self> {
        let channel = block_on(async {
            tonic::transport::Endpoint::from_shared(url.into())
                .context("Invalid coordinator URL")?
                .connect_timeout(connect_timeout)
                .timeout(request_timeout)
                .connect()
                .await
                .context("Failed to connect to coordinator")
        })?;
        Ok(Self {
            inner: ZiskCoordinatorApiClient::new(channel.clone())
                .max_decoding_message_size(128 * 1024 * 1024)
                .max_encoding_message_size(128 * 1024 * 1024),
            ext: ZiskCoordinatorApiExtClient::new(channel)
                .max_decoding_message_size(128 * 1024 * 1024)
                .max_encoding_message_size(128 * 1024 * 1024),
        })
    }

    /// Register a guest ELF and return its content hash id.
    pub fn register_program(&self, elf: Vec<u8>) -> Result<String> {
        block_on(async {
            let mut gw = self.inner.clone();
            let req = RegisterGuestProgramRequestDto { zisk_elf: elf };
            let resp =
                gw.register_guest_program(req).await.context("RegisterGuestProgram RPC failed")?;
            Ok(resp.into_inner().hash_id)
        })
    }

    /// Registers a recurser spec under the SDK-supplied `recurser_id`.
    /// Idempotent for same-content re-registers.
    pub fn register_aggregation_program(
        &self,
        recurser_id: String,
        spec: DomainAggregationProgramSpec,
    ) -> Result<String> {
        block_on(async {
            let mut gw = self.inner.clone();
            let req = RegisterAggregationProgramRequestDto { recurser_id, spec };
            let resp = gw
                .register_aggregation_program(req)
                .await
                .context("RegisterAggregationProgram RPC failed")?;
            Ok(resp.into_inner().recurser_id)
        })
    }

    /// Submit a job of the given kind and return a [`Job`] handle for tracking it.
    pub fn submit_job(&self, kind: DomainJobKind) -> Result<Job> {
        let resp = block_on(async {
            let mut gw = self.inner.clone();
            let resp = gw.job_request(kind).await.context("JobRequest RPC failed")?;
            Ok::<_, anyhow::Error>(resp.into_inner())
        })?;
        Job::new(resp.job_id, self.clone())
    }

    /// Submit an extended job with caller-defined key/value metadata.
    pub fn submit_job_ext(
        &self,
        kind: DomainJobKind,
        metadata: BTreeMap<String, String>,
    ) -> Result<Job> {
        let job_kind = JobKindExt::try_from(kind).map_err(|e| anyhow::anyhow!(e))?;
        let resp = block_on(async {
            let mut gw = self.ext.clone();
            let req = JobRequestExtMessage {
                job_kind: Some(job_kind),
                metadata: metadata.into_iter().collect(),
            };
            let resp = gw.job_request_ext(req).await.context("JobRequestExt RPC failed")?;
            Ok::<_, anyhow::Error>(resp.into_inner())
        })?;
        Job::new(resp.job_id, self.clone())
    }

    /// Cancel a job by its id without first holding a [`Job`] handle.
    /// Returns `true` if the coordinator actually transitioned the job to
    /// cancelled (i.e. the job existed and wasn't already terminal).
    pub fn cancel_job(&self, job_id: Uuid) -> Result<bool> {
        block_on(async {
            let mut gw = self.inner.clone();
            let resp = gw
                .cancel_job(CancelJobRequest { job_id: job_id.to_string() })
                .await
                .map_err(|e| anyhow::anyhow!("CancelJob RPC failed: {e}"))?;
            Ok(resp.into_inner().cancelled)
        })
    }

    /// A clone of the underlying async tonic client, for callers that need to
    /// drive RPCs directly on a runtime.
    pub fn async_client(&self) -> ZiskCoordinatorApiClient<Channel> {
        self.inner.clone()
    }

    /// Open a persistent stdin stream to a running job.
    ///
    /// Returns an [`InputSender`] that can be used to send multiple chunks.
    /// Drop the sender (or call [`InputSender::close`]) to signal EOF.
    pub fn open_input_stream(&self, job_id: Uuid) -> InputSender {
        InputSender::open(job_id, self.inner.clone())
    }

    /// Open a persistent hints stream to a running job.
    ///
    /// Returns an [`InputSender`] that can be used to send multiple chunks.
    /// Drop the sender (or call [`InputSender::close`]) to signal EOF.
    pub fn open_hints_stream(&self, job_id: Uuid) -> InputSender {
        InputSender::open_hints(job_id, self.inner.clone())
    }
}

/// Bridges sync entry-points (`connect`, `submit_job`, `cancel_job`, ...) to
/// the async tonic stack. Requires a multi-thread tokio runtime in scope.
pub(crate) fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}
