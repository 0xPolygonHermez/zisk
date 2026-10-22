//! gRPC transport adapter.
//!
//! [`GrpcAdapter`] implements the tonic-generated `ZiskCoordinatorApi` trait.
//! Its only responsibilities are proto ↔ domain conversion, input validation
//! at the wire boundary, and call-level observability. All business logic lives
//! in [`crate::handler::CoordinatorHandler`].

use std::borrow::Cow;
use std::pin::Pin;
use std::time::{Duration, Instant};

use futures::{Stream, StreamExt};
use tonic::Streaming;
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::backend::{BackendService, DomainJobKind};
use crate::errors::ApiError;
use crate::handler::CoordinatorHandler;
use crate::proto::zisk_coordinator_api_ext_server::ZiskCoordinatorApiExt;
use crate::proto::zisk_coordinator_api_server::ZiskCoordinatorApi;
use crate::proto::*;
use zisk_coordinator_api::dto::{
    RegisterAggregationProgramRequestDto, RegisterGuestProgramRequestDto,
};

const WAIT_TIMEOUT_DEFAULT_SECS: u32 = 5;
const WAIT_TIMEOUT_MIN_SECS: u32 = 1;
const WAIT_TIMEOUT_MAX_SECS: u32 = 3600;

// Bounds on caller-supplied job metadata. Messages may be up to
// `MAX_MESSAGE_SIZE` (128 MB), so metadata is capped here to keep a client from
// pushing large blobs into backend storage and per-worker clones. These are
// generous for label-style metadata and can be raised if a use case needs it.
const METADATA_MAX_ENTRIES: usize = 32;
const METADATA_MAX_KEY_BYTES: usize = 128;
const METADATA_MAX_VALUE_BYTES: usize = 1024;

/// Reject caller-supplied metadata early with `InvalidArgument`, before it
/// reaches backend storage or worker dispatch: on a job kind that does not take
/// it, or when oversized.
fn validate_metadata(
    kind: &DomainJobKind,
    metadata: &std::collections::BTreeMap<String, String>,
) -> Result<(), Status> {
    if metadata.is_empty() {
        return Ok(());
    }
    if let Some(name) = kind_without_metadata(kind) {
        return Err(Status::invalid_argument(format!("{name} jobs do not take metadata")));
    }
    if metadata.len() > METADATA_MAX_ENTRIES {
        return Err(Status::invalid_argument(format!(
            "metadata has too many entries ({} > {METADATA_MAX_ENTRIES})",
            metadata.len()
        )));
    }
    for (k, v) in metadata {
        if k.len() > METADATA_MAX_KEY_BYTES {
            return Err(Status::invalid_argument(format!(
                "metadata key too long ({} > {METADATA_MAX_KEY_BYTES} bytes)",
                k.len()
            )));
        }
        if v.len() > METADATA_MAX_VALUE_BYTES {
            return Err(Status::invalid_argument(format!(
                "metadata value for key {k:?} too long ({} > {METADATA_MAX_VALUE_BYTES} bytes)",
                v.len()
            )));
        }
    }
    Ok(())
}

/// The proto name of a job kind that does not take caller metadata, `None` for
/// the kinds that keep it.
fn kind_without_metadata(kind: &DomainJobKind) -> Option<&'static str> {
    match kind {
        DomainJobKind::Prove(_)
        | DomainJobKind::Execute(_)
        | DomainJobKind::Wrap(_)
        | DomainJobKind::AggregateProofs(_) => None,
        DomainJobKind::Setup(_) => Some("setup"),
        DomainJobKind::SetupAggregationProgram(_) => Some("setup_aggregation_program"),
    }
}

/// Server-streaming type for the `WatchJob` RPC.
pub type WatchJobStream = Pin<Box<dyn Stream<Item = Result<JobEvent, Status>> + Send + 'static>>;

/// gRPC adapter: implements the `ZiskCoordinatorApi` service by converting
/// proto messages to/from domain types and delegating to a [`CoordinatorHandler`].
pub struct GrpcAdapter<B: BackendService> {
    handler: CoordinatorHandler<B>,
}

impl<B: BackendService> GrpcAdapter<B> {
    /// Wrap a handler in the gRPC adapter.
    pub fn new(handler: CoordinatorHandler<B>) -> Self {
        Self { handler }
    }

    /// Submit a job for `JobRequest` and its deprecated alias `JobRequestExt`.
    async fn submit_job(
        &self,
        kind: DomainJobKind,
        metadata: std::collections::BTreeMap<String, String>,
    ) -> Result<Response<JobResponse>, Status> {
        validate_metadata(&kind, &metadata)?;
        self.handler
            .submit_job(kind, metadata)
            .await
            .map(|r| Response::new(JobResponse { job_id: r.job_id.to_string() }))
            .map_err(Status::from)
    }

    /// Run an RPC's body and record the call in the call log and the request metrics: every
    /// outcome, the request's own validation errors included.
    async fn logged<T>(
        method: &'static str,
        body: impl std::future::Future<Output = Result<T, Status>>,
    ) -> Result<T, Status> {
        let start = Instant::now();
        let result = body.await;
        Self::log_call(method, start, result.as_ref().map(|_| ()));
        result
    }

    pub(crate) fn log_call(method: &'static str, start: Instant, result: Result<(), &Status>) {
        let elapsed_secs = start.elapsed().as_secs_f64();
        let elapsed_ms = (elapsed_secs * 1_000.0) as u128;

        let status_label = match result {
            Ok(()) => "ok",
            Err(s) => s.code().description(),
        };

        let method_str: Cow<'static, str> = Cow::Borrowed(method);
        metrics::counter!(
            "coordinator_requests_total",
            "method" => method_str.clone(),
            "status" => status_label
        )
        .increment(1);
        metrics::histogram!("coordinator_request_duration_seconds", "method" => method_str)
            .record(elapsed_secs);

        match result {
            Ok(()) => {
                if elapsed_ms > 5_000 {
                    tracing::debug!(method, elapsed_ms, "slow gRPC call");
                } else {
                    tracing::debug!(method, elapsed_ms, "gRPC call OK");
                }
            }
            Err(status) => {
                let code = status.code() as i32;
                let message = status.message();
                if status.code() == tonic::Code::Internal {
                    error!(method, elapsed_ms, %code, message, "gRPC call failed (internal)");
                } else {
                    info!(method, elapsed_ms, %code, message, "gRPC call failed");
                }
            }
        }
    }
}

#[tonic::async_trait]
impl<B: BackendService> ZiskCoordinatorApi for GrpcAdapter<B> {
    type WatchJobStream = WatchJobStream;

    #[instrument(level = "debug", skip(self, request), fields(elf_bytes))]
    async fn register_guest_program(
        &self,
        request: Request<RegisterGuestProgramRequest>,
    ) -> Result<Response<RegisterGuestProgramResponse>, Status> {
        Self::logged("RegisterGuestProgram", async move {
            let req: RegisterGuestProgramRequestDto = request.into_inner().into();
            tracing::Span::current().record("elf_bytes", req.zisk_elf.len());

            if req.zisk_elf.is_empty() {
                Err(Status::invalid_argument("zisk_elf must not be empty"))
            } else {
                self.handler
                    .register_guest_program(req)
                    .await
                    .map(|dto| Response::new(dto.into()))
                    .map_err(Status::from)
            }
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request))]
    async fn register_aggregation_program(
        &self,
        request: Request<RegisterAggregationProgramRequest>,
    ) -> Result<Response<RegisterAggregationProgramResponse>, Status> {
        Self::logged("RegisterAggregationProgram", async move {
            let req: RegisterAggregationProgramRequestDto =
                request.into_inner().try_into().map_err(|e: String| Status::invalid_argument(e))?;

            self.handler
                .register_aggregation_program(req)
                .await
                .map(|dto| Response::new(dto.into()))
                .map_err(Status::from)
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request))]
    async fn job_request(
        &self,
        request: Request<JobRequestMessage>,
    ) -> Result<Response<JobResponse>, Status> {
        Self::logged("JobRequest", async move {
            let msg = request.into_inner();
            let kind = msg
                .job_kind
                .ok_or_else(|| Status::invalid_argument("job_kind must be set"))?
                .try_into()
                .map_err(|e: String| Status::invalid_argument(e))?;
            self.submit_job(kind, msg.metadata).await
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request), fields(job_id = %request.get_ref().job_id))]
    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<WaitJobResultResponse>, Status> {
        Self::logged("WaitJobResult", async move {
            let req = request.into_inner();
            let job_id = parse_uuid(&req.job_id)?;
            let timeout_secs = req
                .timeout_seconds
                .unwrap_or(WAIT_TIMEOUT_DEFAULT_SECS)
                .clamp(WAIT_TIMEOUT_MIN_SECS, WAIT_TIMEOUT_MAX_SECS);
            let timeout = Duration::from_secs(timeout_secs as u64);

            self.handler
                .wait_job_result(job_id, timeout)
                .await
                .map(|wait| {
                    Response::new(WaitJobResultResponse {
                        job_id: wait.job_id.to_string(),
                        job_status: Some((&wait.job_status).into()),
                        result: wait.result.map(Into::into),
                    })
                })
                .map_err(Status::from)
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request), fields(job_id = %request.get_ref().job_id))]
    async fn watch_job(
        &self,
        request: Request<WatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        Self::logged("WatchJob", async move {
            let job_id = parse_uuid(&request.into_inner().job_id)?;

            self.handler
                .watch_job(job_id)
                .await
                .map(|stream| {
                    let proto_stream = stream.map(|r| r.map(Into::into).map_err(Status::from));
                    Response::new(Box::pin(proto_stream) as WatchJobStream)
                })
                .map_err(Status::from)
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request))]
    async fn push_job_input(
        &self,
        request: Request<Streaming<PushJobInputRequest>>,
    ) -> Result<Response<()>, Status> {
        Self::logged("PushJobInput", async move {
            let mut stream = request.into_inner();

            let first = stream
                .next()
                .await
                .ok_or_else(|| Status::invalid_argument("PushJobInput stream must not be empty"))?
                .map_err(|e| Status::internal(format!("stream read error: {e}")))?;

            let job_id = parse_uuid(&first.job_id)?;
            let first_chunk =
                first.chunk.ok_or_else(|| Status::invalid_argument("chunk must be set"))?.into();

            let chunk_stream =
                futures::stream::once(async move { Ok(first_chunk) }).chain(stream.map(|result| {
                    result
                        .map_err(|e| ApiError::Internal(format!("stream read error: {e}")))
                        .and_then(|msg| {
                            msg.chunk
                                .ok_or_else(|| ApiError::InvalidJobState {
                                    reason: "PushJobInput message missing chunk field".into(),
                                })
                                .map(Into::into)
                        })
                }));

            self.handler
                .push_job_input(job_id, Box::pin(chunk_stream))
                .await
                .map(Response::new)
                .map_err(Status::from)
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request))]
    async fn push_job_hints_input(
        &self,
        request: Request<Streaming<PushJobHintsInputRequest>>,
    ) -> Result<Response<()>, Status> {
        Self::logged("PushJobHintsInput", async move {
            let mut stream = request.into_inner();

            let first = stream
                .next()
                .await
                .ok_or_else(|| {
                    Status::invalid_argument("PushJobHintsInput stream must not be empty")
                })?
                .map_err(|e| Status::internal(format!("stream read error: {e}")))?;

            let job_id = parse_uuid(&first.job_id)?;
            let first_chunk =
                first.chunk.ok_or_else(|| Status::invalid_argument("chunk must be set"))?.into();

            let chunk_stream =
                futures::stream::once(async move { Ok(first_chunk) }).chain(stream.map(|result| {
                    result
                        .map_err(|e| ApiError::Internal(format!("stream read error: {e}")))
                        .and_then(|msg| {
                            msg.chunk
                                .ok_or_else(|| ApiError::InvalidJobState {
                                    reason: "PushJobHintsInput message missing chunk field".into(),
                                })
                                .map(Into::into)
                        })
                }));

            self.handler
                .push_job_hints_input(job_id, Box::pin(chunk_stream))
                .await
                .map(Response::new)
                .map_err(Status::from)
        })
        .await
    }

    #[instrument(level = "debug", skip(self, request), fields(job_id = %request.get_ref().job_id))]
    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        Self::logged("CancelJob", async move {
            let job_id = parse_uuid(&request.into_inner().job_id)?;

            self.handler
                .cancel_job(job_id)
                .await
                .map(|cancelled| {
                    Response::new(CancelJobResponse { job_id: job_id.to_string(), cancelled })
                })
                .map_err(Status::from)
        })
        .await
    }
}

/// Deprecated alias of `JobRequest`, kept for clients built against the extended
/// API until the first non-alpha release.
#[allow(deprecated)]
#[tonic::async_trait]
impl<B: BackendService> ZiskCoordinatorApiExt for GrpcAdapter<B> {
    #[instrument(level = "debug", skip(self, request))]
    async fn job_request_ext(
        &self,
        request: Request<JobRequestExtMessage>,
    ) -> Result<Response<JobResponse>, Status> {
        Self::logged("JobRequestExt", async move {
            let msg = request.into_inner();
            let kind = msg
                .job_kind
                .ok_or_else(|| Status::invalid_argument("job_kind must be set"))?
                .try_into()
                .map_err(|e: String| Status::invalid_argument(e))?;

            self.submit_job(kind, msg.metadata.into_iter().collect()).await
        })
        .await
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid UUID: {s}")))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use metrics_exporter_prometheus::PrometheusBuilder;

    use super::*;
    use crate::backend::mock::MockBackend;

    /// A request its own validation refuses is still recorded: a `?` used to return ahead of
    /// the call log, so malformed requests never reached `coordinator_requests_total`.
    #[test]
    fn a_request_refused_by_its_validation_is_recorded() {
        let recorder = PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        let adapter = GrpcAdapter::new(CoordinatorHandler::new(Arc::new(MockBackend::default())));

        metrics::with_local_recorder(&recorder, || {
            futures::executor::block_on(async {
                let no_job_kind = adapter.job_request(Request::new(JobRequestMessage::default()));
                assert_eq!(no_job_kind.await.unwrap_err().code(), tonic::Code::InvalidArgument);
                let bad_job_id = adapter
                    .cancel_job(Request::new(CancelJobRequest { job_id: "not-a-uuid".into() }));
                assert_eq!(bad_job_id.await.unwrap_err().code(), tonic::Code::InvalidArgument);
            })
        });

        let rendered = handle.render();
        for method in ["JobRequest", "CancelJob"] {
            assert!(
                rendered.lines().any(|l| l.starts_with("coordinator_requests_total")
                    && l.contains(&format!("method=\"{method}\""))
                    && l.ends_with(" 1")),
                "{method} not recorded:\n{rendered}"
            );
        }
    }
}
