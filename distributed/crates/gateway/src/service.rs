//! gRPC service implementation.
//!
//! [`GatewayService`] implements the tonic-generated `ZiskGatewayApiServer`
//! trait, delegating all business logic to a [`BackendService`].

use crate::backend::BackendService;
use crate::errors::GatewayError;
use crate::proto::zisk_gateway_api_server::ZiskGatewayApi;
use crate::proto::*;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tonic::Streaming;
use tonic::{Request, Response, Status};
use tracing::instrument;
use tracing::{error, info, warn};
use uuid::Uuid;

const WAIT_TIMEOUT_DEFAULT_SECS: u32 = 5;
const WAIT_TIMEOUT_MIN_SECS: u32 = 1;
const WAIT_TIMEOUT_MAX_SECS: u32 = 3600;

// ── Stream type for WatchJob ──────────────────────────────────────────────────

pub type WatchJobStream = Pin<Box<dyn Stream<Item = Result<JobEvent, Status>> + Send + 'static>>;

pub struct GatewayService<B: BackendService> {
    pub(crate) backend: Arc<B>,
}

impl<B: BackendService> GatewayService<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }

    /// Log a completed RPC call and update Prometheus counters.
    pub(crate) fn log_call(method: &str, start: Instant, result: Result<(), &Status>) {
        let elapsed_secs = start.elapsed().as_secs_f64();
        let elapsed_ms = (elapsed_secs * 1_000.0) as u128;

        let status_label = match result {
            Ok(()) => "ok",
            Err(s) => s.code().description(),
        };

        metrics::counter!("gateway_requests_total", "method" => method.to_owned(), "status" => status_label).increment(1);
        metrics::histogram!("gateway_request_duration_seconds", "method" => method.to_owned())
            .record(elapsed_secs);

        match result {
            Ok(()) => {
                if elapsed_ms > 5_000 {
                    warn!(method, elapsed_ms, "slow gRPC call");
                } else {
                    tracing::debug!(method, elapsed_ms, "gRPC call OK");
                }
            }
            Err(status) => {
                let code = status.code() as i32;
                if status.code() == tonic::Code::Internal {
                    error!(method, elapsed_ms, %code, "gRPC call failed (internal)");
                } else {
                    info!(method, elapsed_ms, %code, "gRPC call failed");
                }
            }
        }
    }

    #[instrument(skip(self, req), fields(elf_bytes = req.zisk_elf.len()))]
    pub(crate) async fn handle_register_guest_program(
        &self,
        req: RegisterGuestProgramRequest,
    ) -> Result<RegisterGuestProgramResponse, Status> {
        if req.zisk_elf.is_empty() {
            return Err(Status::invalid_argument("zisk_elf must not be empty"));
        }

        let hash_id =
            self.backend.register_guest_program(req.zisk_elf).await.map_err(Status::from)?;

        Ok(RegisterGuestProgramResponse { hash_id })
    }

    #[instrument(skip(self, req))]
    pub(crate) async fn handle_job_request(
        &self,
        req: JobRequestMessage,
    ) -> Result<JobResponse, Status> {
        let kind = req
            .job_kind
            .ok_or_else(|| Status::invalid_argument("job_kind must be set"))?
            .try_into()
            .map_err(|e: String| Status::invalid_argument(e))?;
        let job_id = self.backend.submit_job(kind).await.map_err(into_status)?;

        Ok(JobResponse { job_id: job_id.to_string() })
    }

    #[instrument(skip(self, req), fields(job_id = %req.job_id))]
    pub(crate) async fn handle_wait_job_result(
        &self,
        req: WaitJobResultRequest,
    ) -> Result<WaitJobResultResponse, Status> {
        let job_id = parse_uuid(&req.job_id)?;
        let timeout_secs = req
            .timeout_seconds
            .unwrap_or(WAIT_TIMEOUT_DEFAULT_SECS)
            .clamp(WAIT_TIMEOUT_MIN_SECS, WAIT_TIMEOUT_MAX_SECS);
        let timeout = Duration::from_secs(timeout_secs as u64);

        let wait = self.backend.wait_job_result(job_id, timeout).await.map_err(into_status)?;

        Ok(WaitJobResultResponse {
            job_id: wait.job_id.to_string(),
            job_status: Some((&wait.job_status).into()),
            result: wait.result.map(Into::into),
        })
    }

    #[instrument(skip(self, req), fields(job_id = %req.job_id))]
    pub(crate) async fn handle_watch_job(
        &self,
        req: WatchJobRequest,
    ) -> Result<WatchJobStream, Status> {
        let job_id = parse_uuid(&req.job_id)?;
        let event_stream = self.backend.watch_job(job_id).await.map_err(into_status)?;
        let proto_stream = event_stream.map(|result| result.map(Into::into).map_err(into_status));
        Ok(Box::pin(proto_stream))
    }

    pub(crate) async fn handle_push_job_input(
        &self,
        mut stream: Streaming<PushJobInputRequest>,
    ) -> Result<(), Status> {
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
                    .map_err(|e| GatewayError::Internal(format!("stream read error: {e}")))
                    .and_then(|msg| {
                        msg.chunk
                            .ok_or_else(|| GatewayError::InvalidJobState {
                                reason: "PushJobInput message missing chunk field".into(),
                            })
                            .map(Into::into)
                    })
            }));

        self.backend.push_job_input(job_id, Box::pin(chunk_stream)).await.map_err(into_status)
    }

    #[instrument(skip(self, req), fields(job_id = %req.job_id))]
    pub(crate) async fn handle_cancel_job(
        &self,
        req: CancelJobRequest,
    ) -> Result<CancelJobResponse, Status> {
        let job_id = parse_uuid(&req.job_id)?;
        let cancelled = self.backend.cancel_job(job_id).await.map_err(into_status)?;
        Ok(CancelJobResponse { job_id: job_id.to_string(), cancelled })
    }
}

#[tonic::async_trait]
impl<B: BackendService> ZiskGatewayApi for GatewayService<B> {
    type WatchJobStream = WatchJobStream;

    async fn register_guest_program(
        &self,
        request: Request<RegisterGuestProgramRequest>,
    ) -> Result<Response<RegisterGuestProgramResponse>, Status> {
        let start = Instant::now();
        let result = self.handle_register_guest_program(request.into_inner()).await;
        Self::log_call("RegisterGuestProgram", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn job_request(
        &self,
        request: Request<JobRequestMessage>,
    ) -> Result<Response<JobResponse>, Status> {
        let start = Instant::now();
        let result = self.handle_job_request(request.into_inner()).await;
        Self::log_call("JobRequest", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<WaitJobResultResponse>, Status> {
        let start = Instant::now();
        let result = self.handle_wait_job_result(request.into_inner()).await;
        Self::log_call("WaitJobResult", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }
    async fn watch_job(
        &self,
        request: Request<WatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let start = Instant::now();
        let result = self.handle_watch_job(request.into_inner()).await;
        Self::log_call("WatchJob", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn push_job_input(
        &self,
        request: Request<tonic::Streaming<PushJobInputRequest>>,
    ) -> Result<Response<()>, Status> {
        let start = Instant::now();
        let result = self.handle_push_job_input(request.into_inner()).await;
        Self::log_call("PushJobInput", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let start = Instant::now();
        let result = self.handle_cancel_job(request.into_inner()).await;
        Self::log_call("CancelJob", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }
}

// ── Misc helpers ──────────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid UUID: {s}")))
}

fn into_status(e: GatewayError) -> Status {
    e.into()
}
