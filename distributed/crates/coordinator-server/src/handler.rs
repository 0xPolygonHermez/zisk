//! Transport-agnostic coordinator handler.
//!
//! [`CoordinatorHandler`] contains all business logic, operating exclusively on
//! domain types. Transport adapters ([`crate::grpc::GrpcAdapter`],
//! own the format-conversion layer and delegate here.

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::backend::{
    BackendService, DomainJobKind, InputChunkStream, JobEventStream,
    RegisterGuestProgramRequestDto, RegisterGuestProgramResponseDto,
    RegisterRecurserAggregatorRequestDto, RegisterRecurserAggregatorResponseDto, WaitResult,
};

use zisk_coordinator_api::dto::SubmitJobResult;

use crate::errors::ApiResult;

pub struct CoordinatorHandler<B: BackendService> {
    backend: Arc<B>,
}

impl<B: BackendService> CoordinatorHandler<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }

    pub async fn register_guest_program(
        &self,
        req: RegisterGuestProgramRequestDto,
    ) -> ApiResult<RegisterGuestProgramResponseDto> {
        let hash_id = self.backend.register_guest_program(req.zisk_elf).await?;
        Ok(RegisterGuestProgramResponseDto { hash_id })
    }

    pub async fn register_recurser_aggregator(
        &self,
        req: RegisterRecurserAggregatorRequestDto,
    ) -> ApiResult<RegisterRecurserAggregatorResponseDto> {
        let recurser_id =
            self.backend.register_recurser_aggregator(req.recurser_id, req.spec).await?;
        Ok(RegisterRecurserAggregatorResponseDto { recurser_id })
    }

    pub async fn submit_job(&self, job: DomainJobKind) -> ApiResult<SubmitJobResult> {
        self.backend.submit_job(job).await
    }

    pub async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> ApiResult<WaitResult> {
        self.backend.wait_job_result(job_id, timeout).await
    }

    pub async fn watch_job(&self, job_id: Uuid) -> ApiResult<JobEventStream> {
        self.backend.watch_job(job_id).await
    }

    pub async fn push_job_input(&self, job_id: Uuid, chunks: InputChunkStream) -> ApiResult<()> {
        self.backend.push_job_input(job_id, chunks).await
    }

    pub async fn push_job_hints_input(
        &self,
        job_id: Uuid,
        chunks: InputChunkStream,
    ) -> ApiResult<()> {
        self.backend.push_job_hints_input(job_id, chunks).await
    }

    pub async fn cancel_job(&self, job_id: Uuid) -> ApiResult<bool> {
        self.backend.cancel_job(job_id).await
    }
}
