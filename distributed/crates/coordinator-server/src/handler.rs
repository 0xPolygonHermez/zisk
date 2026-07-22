//! Transport-agnostic coordinator handler.
//!
//! [`CoordinatorHandler`] contains all business logic, operating exclusively on
//! domain types. Transport adapters — currently just [`crate::grpc::GrpcAdapter`] —
//! own the format-conversion layer and delegate here.

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::backend::{
    BackendService, DomainJobKind, InputChunkStream, JobEventStream,
    RegisterAggregationProgramRequestDto, RegisterAggregationProgramResponseDto,
    RegisterGuestProgramRequestDto, RegisterGuestProgramResponseDto, WaitResult,
};

use zisk_coordinator_api::dto::SubmitJobResult;

use crate::errors::ApiResult;

/// Transport-agnostic coordinator business logic over a [`BackendService`].
pub struct CoordinatorHandler<B: BackendService> {
    backend: Arc<B>,
}

impl<B: BackendService> CoordinatorHandler<B> {
    /// Create a handler backed by the given backend.
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }

    /// Register a guest ELF and return its content hash id.
    pub async fn register_guest_program(
        &self,
        req: RegisterGuestProgramRequestDto,
    ) -> ApiResult<RegisterGuestProgramResponseDto> {
        let hash_id = self.backend.register_guest_program(req.zisk_elf).await?;
        Ok(RegisterGuestProgramResponseDto { hash_id })
    }

    /// Register an aggregation (recurser) program and return its id.
    pub async fn register_aggregation_program(
        &self,
        req: RegisterAggregationProgramRequestDto,
    ) -> ApiResult<RegisterAggregationProgramResponseDto> {
        let recurser_id =
            self.backend.register_aggregation_program(req.recurser_id, req.spec).await?;
        Ok(RegisterAggregationProgramResponseDto { recurser_id })
    }

    /// Submit a base job and return its assigned id.
    pub async fn submit_job(&self, job: DomainJobKind) -> ApiResult<SubmitJobResult> {
        self.backend.submit_job(job).await
    }

    /// Submit a job with caller-defined key/value metadata.
    pub async fn submit_job_ext(
        &self,
        job: DomainJobKind,
        metadata: std::collections::BTreeMap<String, String>,
    ) -> ApiResult<SubmitJobResult> {
        self.backend.submit_job_ext(job, metadata).await
    }

    /// Wait up to `timeout` for the job to reach a terminal state.
    pub async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> ApiResult<WaitResult> {
        self.backend.wait_job_result(job_id, timeout).await
    }

    /// Stream lifecycle events for a running job.
    pub async fn watch_job(&self, job_id: Uuid) -> ApiResult<JobEventStream> {
        self.backend.watch_job(job_id).await
    }

    /// Stream stdin chunks to a running job.
    pub async fn push_job_input(&self, job_id: Uuid, chunks: InputChunkStream) -> ApiResult<()> {
        self.backend.push_job_input(job_id, chunks).await
    }

    /// Stream hint chunks to a running job.
    pub async fn push_job_hints_input(
        &self,
        job_id: Uuid,
        chunks: InputChunkStream,
    ) -> ApiResult<()> {
        self.backend.push_job_hints_input(job_id, chunks).await
    }

    /// Cancel a job; returns `true` if it was actually transitioned to cancelled.
    pub async fn cancel_job(&self, job_id: Uuid) -> ApiResult<bool> {
        self.backend.cancel_job(job_id).await
    }
}
