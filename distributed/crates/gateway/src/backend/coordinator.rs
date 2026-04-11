//! Coordinator backend — phase 2, not yet implemented.
//!
//! This backend will forward all calls to a real `zisk-coordinator` instance
//! over gRPC once the coordinator has been extended with the gateway RPCs.
//!
//! All methods currently panic with `unimplemented!`. To activate:
//! 1. Wire up a tonic client for the coordinator's new gateway-facing service.
//! 2. Replace each `unimplemented!` with the actual RPC call and response mapping.
//! 3. Update `BackendMode::Coordinator` handling in `src/cli/main.rs`.

use std::time::Duration;

use async_trait::async_trait;
use uuid::Uuid;

use super::{BackendService, DomainJobKind, InputChunkStream, JobEventStream, WaitResult};
use crate::errors::GatewayResult;

pub struct CoordinatorBackend;

impl CoordinatorBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CoordinatorBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BackendService for CoordinatorBackend {
    async fn register_guest_program(&self, _elf: Vec<u8>) -> GatewayResult<String> {
        unimplemented!("CoordinatorBackend::register_guest_program — phase 2")
    }

    async fn submit_job(&self, _kind: DomainJobKind) -> GatewayResult<Uuid> {
        unimplemented!("CoordinatorBackend::submit_job — phase 2")
    }

    async fn wait_job_result(
        &self,
        _job_id: Uuid,
        _timeout: Duration,
    ) -> GatewayResult<WaitResult> {
        unimplemented!("CoordinatorBackend::wait_job_result — phase 2")
    }

    async fn watch_job(&self, _job_id: Uuid) -> GatewayResult<JobEventStream> {
        unimplemented!("CoordinatorBackend::watch_job — phase 2")
    }

    async fn push_job_input(&self, _job_id: Uuid, _chunks: InputChunkStream) -> GatewayResult<()> {
        unimplemented!("CoordinatorBackend::push_job_input — phase 2")
    }

    async fn cancel_job(&self, _job_id: Uuid) -> GatewayResult<bool> {
        unimplemented!("CoordinatorBackend::cancel_job — phase 2")
    }
}
