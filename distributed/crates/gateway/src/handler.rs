//! Transport-agnostic gateway handler.
//!
//! [`GatewayHandler`] contains all business logic, operating exclusively on
//! domain types. Transport adapters ([`crate::grpc::GrpcAdapter`],
//! own the format-conversion layer and delegate here.

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::backend::{
    BackendService, DomainJobKind, InputChunkStream, JobEventStream,
    RegisterGuestProgramRequestDto, RegisterGuestProgramResponseDto, WaitResult,
};
use crate::errors::GatewayResult;

pub struct GatewayHandler<B: BackendService> {
    backend: Arc<B>,
}

impl<B: BackendService> GatewayHandler<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self { backend }
    }

    pub async fn register_guest_program(
        &self,
        req: RegisterGuestProgramRequestDto,
    ) -> GatewayResult<RegisterGuestProgramResponseDto> {
        let hash_id = self.backend.register_guest_program(req.zisk_elf).await?;
        Ok(RegisterGuestProgramResponseDto { hash_id })
    }

    pub async fn submit_job(&self, job: DomainJobKind) -> GatewayResult<Uuid> {
        self.backend.submit_job(job).await
    }

    pub async fn wait_job_result(
        &self,
        job_id: Uuid,
        timeout: Duration,
    ) -> GatewayResult<WaitResult> {
        self.backend.wait_job_result(job_id, timeout).await
    }

    pub async fn watch_job(&self, job_id: Uuid) -> GatewayResult<JobEventStream> {
        self.backend.watch_job(job_id).await
    }

    pub async fn push_job_input(
        &self,
        job_id: Uuid,
        chunks: InputChunkStream,
    ) -> GatewayResult<()> {
        self.backend.push_job_input(job_id, chunks).await
    }

    pub async fn cancel_job(&self, job_id: Uuid) -> GatewayResult<bool> {
        self.backend.cancel_job(job_id).await
    }
}
