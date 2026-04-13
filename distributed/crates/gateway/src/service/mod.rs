//! gRPC service implementation.
//!
//! [`GatewayService`] implements the tonic-generated `ZiskGatewayApiServer`
//! trait, delegating all business logic to a [`BackendService`].
//!
//! # Structure
//!
//! - `mod.rs`   — service struct, tonic trait impl entry points, logging middleware
//! - `programs.rs` — `RegisterGuestProgram` handler + proto↔domain conversions
//! - `jobs.rs`  — all job-related handlers + proto↔domain conversions

pub mod jobs;
pub mod programs;

use std::sync::Arc;
use std::time::Instant;

use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

use crate::backend::BackendService;
use crate::proto::zisk_gateway_api_server::ZiskGatewayApi;
use crate::proto::*;

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
}

#[tonic::async_trait]
impl<B: BackendService> ZiskGatewayApi for GatewayService<B> {
    async fn register_guest_program(
        &self,
        request: Request<RegisterGuestProgramRequest>,
    ) -> Result<Response<RegisterGuestProgramResponse>, Status> {
        let start = Instant::now();
        let result =
            programs::handle_register_guest_program(&self.backend, request.into_inner()).await;
        Self::log_call("RegisterGuestProgram", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn job_request(
        &self,
        request: Request<JobRequestMessage>,
    ) -> Result<Response<JobResponse>, Status> {
        let start = Instant::now();
        let result = jobs::handle_job_request(&self.backend, request.into_inner()).await;
        Self::log_call("JobRequest", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn wait_job_result(
        &self,
        request: Request<WaitJobResultRequest>,
    ) -> Result<Response<WaitJobResultResponse>, Status> {
        let start = Instant::now();
        let result = jobs::handle_wait_job_result(&self.backend, request.into_inner()).await;
        Self::log_call("WaitJobResult", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    type WatchJobStream = jobs::WatchJobStream;

    async fn watch_job(
        &self,
        request: Request<WatchJobRequest>,
    ) -> Result<Response<Self::WatchJobStream>, Status> {
        let start = Instant::now();
        let result = jobs::handle_watch_job(&self.backend, request.into_inner()).await;
        Self::log_call("WatchJob", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn push_job_input(
        &self,
        request: Request<tonic::Streaming<PushJobInputRequest>>,
    ) -> Result<Response<()>, Status> {
        let start = Instant::now();
        let result = jobs::handle_push_job_input(&self.backend, request.into_inner()).await;
        Self::log_call("PushJobInput", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }

    async fn cancel_job(
        &self,
        request: Request<CancelJobRequest>,
    ) -> Result<Response<CancelJobResponse>, Status> {
        let start = Instant::now();
        let result = jobs::handle_cancel_job(&self.backend, request.into_inner()).await;
        Self::log_call("CancelJob", start, result.as_ref().map(|_| ()));
        result.map(Response::new)
    }
}
