//! gRPC transport adapter.
//!
//! [`GrpcAdapter`] implements the tonic-generated [`ZiskGatewayApi`] trait.
//! Its only responsibilities are proto ↔ domain conversion, input validation
//! at the wire boundary, and call-level observability. All business logic lives
//! in [`crate::handler::GatewayHandler`].

use std::borrow::Cow;
use std::pin::Pin;
use std::time::{Duration, Instant};

use futures::{Stream, StreamExt};
use tonic::Streaming;
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::backend::BackendService;
use crate::errors::GatewayError;
use crate::handler::GatewayHandler;
use crate::proto::zisk_gateway_api_server::ZiskGatewayApi;
use crate::proto::*;
use zisk_gateway_api::dto::RegisterGuestProgramRequestDto;

const WAIT_TIMEOUT_DEFAULT_SECS: u32 = 5;
const WAIT_TIMEOUT_MIN_SECS: u32 = 1;
const WAIT_TIMEOUT_MAX_SECS: u32 = 3600;

pub type WatchJobStream = Pin<Box<dyn Stream<Item = Result<JobEvent, Status>> + Send + 'static>>;

pub struct GrpcAdapter<B: BackendService> {
    handler: GatewayHandler<B>,
}

impl<B: BackendService> GrpcAdapter<B> {
    pub fn new(handler: GatewayHandler<B>) -> Self {
        Self { handler }
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
            "gateway_requests_total",
            "method" => method_str.clone(),
            "status" => status_label
        )
        .increment(1);
        metrics::histogram!("gateway_request_duration_seconds", "method" => method_str)
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
}

#[tonic::async_trait]
impl<B: BackendService> ZiskGatewayApi for GrpcAdapter<B> {
    type WatchJobStream = WatchJobStream;

    #[instrument(skip(self, request), fields(elf_bytes))]
    async fn register_guest_program(
        &self,
        request: Request<RegisterGuestProgramRequest>,
    ) -> Result<Response<RegisterGuestProgramResponse>, Status> {
        let start = Instant::now();
        let req: RegisterGuestProgramRequestDto = request.into_inner().into();
        tracing::Span::current().record("elf_bytes", req.zisk_elf.len());

        let result = if req.zisk_elf.is_empty() {
            Err(Status::invalid_argument("zisk_elf must not be empty"))
        } else {
            self.handler
                .register_guest_program(req)
                .await
                .map(|dto| Response::new(dto.into()))
                .map_err(Status::from)
        };

        Self::log_call("RegisterGuestProgram", start, result.as_ref().map(|_| ()));
        result
    }

    #[instrument(skip(self, request))]
    async fn job_request(
        &self,
        request: Request<JobRequestMessage>,
    ) -> Result<Response<JobResponse>, Status> {
        let start = Instant::now();
        let kind = request
            .into_inner()
            .job_kind
            .ok_or_else(|| Status::invalid_argument("job_kind must be set"))?
            .try_into()
            .map_err(|e: String| Status::invalid_argument(e))?;

        let result = self
            .handler
            .submit_job(kind)
            .await
            .map(|job_id| Response::new(JobResponse { job_id: job_id.to_string() }))
            .map_err(Status::from);

        Self::log_call("JobRequest", start, result.as_ref().map(|_| ()));
        result
    }

    #[instrument(skip(self, request), fields(job_id = %request.get_ref().job_id))]
    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<WaitJobResultResponse>, Status> {
        let start = Instant::now();
        let req = request.into_inner();
        let job_id = parse_uuid(&req.job_id)?;
        let timeout_secs = req
            .timeout_seconds
            .unwrap_or(WAIT_TIMEOUT_DEFAULT_SECS)
            .clamp(WAIT_TIMEOUT_MIN_SECS, WAIT_TIMEOUT_MAX_SECS);
        let timeout = Duration::from_secs(timeout_secs as u64);

        let result = self
            .handler
            .wait_job_result(job_id, timeout)
            .await
            .map(|wait| {
                Response::new(WaitJobResultResponse {
                    job_id: wait.job_id.to_string(),
                    job_status: Some((&wait.job_status).into()),
                    result: wait.result.map(Into::into),
                })
            })
            .map_err(Status::from);

        Self::log_call("WaitJobResult", start, result.as_ref().map(|_| ()));
        result
    }

    #[instrument(skip(self, request), fields(job_id = %request.get_ref().job_id))]
    async fn watch_job(
        &self,
        request: Request<WatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let start = Instant::now();
        let job_id = parse_uuid(&request.into_inner().job_id)?;

        let result = self
            .handler
            .watch_job(job_id)
            .await
            .map(|stream| {
                let proto_stream = stream.map(|r| r.map(Into::into).map_err(Status::from));
                Response::new(Box::pin(proto_stream) as WatchJobStream)
            })
            .map_err(Status::from);

        Self::log_call("WatchJob", start, result.as_ref().map(|_| ()));
        result
    }

    #[instrument(skip(self, request))]
    async fn push_job_input(
        &self,
        request: Request<Streaming<PushJobInputRequest>>,
    ) -> Result<Response<()>, Status> {
        let start = Instant::now();
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
                    .map_err(|e| GatewayError::Internal(format!("stream read error: {e}")))
                    .and_then(|msg| {
                        msg.chunk
                            .ok_or_else(|| GatewayError::InvalidJobState {
                                reason: "PushJobInput message missing chunk field".into(),
                            })
                            .map(Into::into)
                    })
            }));

        let result = self
            .handler
            .push_job_input(job_id, Box::pin(chunk_stream))
            .await
            .map(Response::new)
            .map_err(Status::from);

        Self::log_call("PushJobInput", start, result.as_ref().map(|_| ()));
        result
    }

    #[instrument(skip(self, request), fields(job_id = %request.get_ref().job_id))]
    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let start = Instant::now();
        let job_id = parse_uuid(&request.into_inner().job_id)?;

        let result = self
            .handler
            .cancel_job(job_id)
            .await
            .map(|cancelled| {
                Response::new(CancelJobResponse { job_id: job_id.to_string(), cancelled })
            })
            .map_err(Status::from);

        Self::log_call("CancelJob", start, result.as_ref().map(|_| ()));
        result
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::invalid_argument(format!("invalid UUID: {s}")))
}
