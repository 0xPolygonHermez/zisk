use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration;

use anyhow::{Context, Result};
use tonic::transport::Channel;
use tonic::{Response, Status};
use uuid::Uuid;
use zisk_coordinator_api::dto::{
    DomainAggregationProgramSpec, DomainJobKind, RegisterAggregationProgramRequestDto,
    RegisterGuestProgramRequestDto,
};
use zisk_coordinator_api::grpc::proto::{CancelJobRequest, JobRequestMessage};
use zisk_coordinator_api::grpc::{ZiskCoordinatorApiClient, ZiskCoordinatorApiExtClient};

use crate::error::is_retryable;
use crate::input_sender::InputSender;
use crate::job::Job;

/// How [`CoordinatorClient`] submits a job again when the coordinator refuses it as
/// [retryable](crate::ApiError::is_retryable): it waits `initial_backoff`, doubling each time up to
/// `max_backoff`, for at most `max_attempts` attempts in all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Attempts in all, the first one included.
    pub max_attempts: u32,
    /// The wait before the second attempt.
    pub initial_backoff: Duration,
    /// The longest wait between two attempts.
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    /// 10 attempts over about a minute: 0.5 s, doubling up to 10 s.
    fn default() -> Self {
        Self {
            max_attempts: 10,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    /// The wait before retry number `retry` (0 for the first retry).
    fn backoff(&self, retry: u32) -> Duration {
        self.initial_backoff.saturating_mul(2u32.saturating_pow(retry)).min(self.max_backoff)
    }
}

/// Synchronous client for the ZisK coordinator gRPC API.
///
/// Wraps the async tonic client and exposes blocking methods (register
/// programs, submit/cancel jobs, open input/hints streams). Cheap to
/// [`Clone`] — clones share the same underlying channel.
#[derive(Clone)]
pub struct CoordinatorClient {
    inner: ZiskCoordinatorApiClient<Channel>,
    ext: ZiskCoordinatorApiExtClient<Channel>,
    retry: Option<RetryPolicy>,
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
            retry: None,
        })
    }

    /// Submit jobs again under `policy` while the coordinator refuses them as retryable (busy,
    /// setting up, no ready workers). Without it a job is submitted once.
    ///
    /// While it retries, the client holds a copy of the job's request, inline input included.
    #[must_use]
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = Some(policy);
        self
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
        self.submit_job_with_metadata(kind, BTreeMap::new())
    }

    /// Submit a job of the given kind with caller-defined key/value metadata
    /// (empty for none) and return a [`Job`] handle for tracking it.
    ///
    /// A coordinator that predates job metadata on `JobRequest` runs the job
    /// without it.
    pub fn submit_job_with_metadata(
        &self,
        kind: DomainJobKind,
        metadata: BTreeMap<String, String>,
    ) -> Result<Job> {
        let resp = block_on(async {
            let req = JobRequestMessage { job_kind: Some(kind.into()), metadata };
            let gw = self.inner.clone();
            send(self.retry, req, |req| {
                let mut gw = gw.clone();
                async move { gw.job_request(req).await }
            })
            .await
            .context("JobRequest RPC failed")
        })?;
        Job::new(resp.job_id, self.clone())
    }

    /// Submit a prove or execute job with caller-defined key/value metadata over
    /// the extended API's `JobRequestExt`, which coordinators that predate
    /// metadata on `JobRequest` also take.
    #[deprecated(
        since = "1.4.0-alpha",
        note = "use `submit_job_with_metadata`, which sends the metadata on `JobRequest`; \
                removed in the first non-alpha release"
    )]
    #[allow(deprecated)]
    pub fn submit_job_ext(
        &self,
        kind: DomainJobKind,
        metadata: BTreeMap<String, String>,
    ) -> Result<Job> {
        use zisk_coordinator_api::grpc::proto::{JobKindExt, JobRequestExtMessage};

        let job_kind = JobKindExt::try_from(kind).map_err(|e| anyhow::anyhow!(e))?;
        let resp = block_on(async {
            let req = JobRequestExtMessage {
                job_kind: Some(job_kind),
                metadata: metadata.into_iter().collect(),
            };
            let gw = self.ext.clone();
            send(self.retry, req, |req| {
                let mut gw = gw.clone();
                async move { gw.job_request_ext(req).await }
            })
            .await
            .context("JobRequestExt RPC failed")
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

/// Sends `request` through `call`, again after a backoff while the coordinator refuses it as
/// retryable and `retry` allows another attempt. Without a policy the request is sent once, and
/// never copied.
async fn send<Req, Resp, Fut>(
    retry: Option<RetryPolicy>,
    request: Req,
    mut call: impl FnMut(Req) -> Fut,
) -> Result<Resp, Status>
where
    Req: Clone,
    Fut: Future<Output = Result<Response<Resp>, Status>>,
{
    let Some(policy) = retry else {
        return call(request).await.map(Response::into_inner);
    };
    let mut retry = 0;
    loop {
        match call(request.clone()).await {
            Err(status) if is_retryable(status.code()) && retry + 1 < policy.max_attempts => {
                tokio::time::sleep(policy.backoff(retry)).await;
                retry += 1;
            }
            result => return result.map(Response::into_inner),
        }
    }
}

/// Bridges sync entry-points (`connect`, `submit_job`, `cancel_job`, ...) to
/// the async tonic stack. Requires a multi-thread tokio runtime in scope.
pub(crate) fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use tonic::Code;

    use super::*;

    const NO_WAIT: RetryPolicy = RetryPolicy {
        max_attempts: 3,
        initial_backoff: Duration::ZERO,
        max_backoff: Duration::ZERO,
    };

    /// Answers `answers` in turn, counting the calls.
    async fn send_answering(
        retry: Option<RetryPolicy>,
        answers: &[Code],
        calls: &Cell<usize>,
    ) -> Result<(), Status> {
        send(retry, (), |()| {
            let n = calls.get();
            calls.set(n + 1);
            let answer = answers.get(n).copied().unwrap_or(Code::Ok);
            async move {
                match answer {
                    Code::Ok => Ok(Response::new(())),
                    code => Err(Status::new(code, "refused")),
                }
            }
        })
        .await
    }

    #[tokio::test]
    async fn without_a_policy_a_busy_refusal_is_returned_at_once() {
        let calls = Cell::new(0);
        let err = send_answering(None, &[Code::Unavailable], &calls).await.unwrap_err();
        assert_eq!(err.code(), Code::Unavailable);
        assert_eq!(calls.get(), 1);
    }

    #[tokio::test]
    async fn a_busy_refusal_is_retried_until_accepted() {
        let calls = Cell::new(0);
        send_answering(Some(NO_WAIT), &[Code::Unavailable, Code::Unavailable], &calls)
            .await
            .unwrap();
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test]
    async fn retries_stop_after_max_attempts() {
        let calls = Cell::new(0);
        let busy = [Code::Unavailable; 5];
        let err = send_answering(Some(NO_WAIT), &busy, &calls).await.unwrap_err();
        assert_eq!(err.code(), Code::Unavailable);
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test]
    async fn a_refusal_a_retry_cannot_fix_is_not_retried() {
        let calls = Cell::new(0);
        let err =
            send_answering(Some(NO_WAIT), &[Code::FailedPrecondition], &calls).await.unwrap_err();
        assert_eq!(err.code(), Code::FailedPrecondition);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn backoff_doubles_up_to_its_cap() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.backoff(0), Duration::from_millis(500));
        assert_eq!(policy.backoff(1), Duration::from_secs(1));
        assert_eq!(policy.backoff(4), Duration::from_secs(8));
        assert_eq!(policy.backoff(5), Duration::from_secs(10));
        assert_eq!(policy.backoff(40), Duration::from_secs(10));
    }
}
