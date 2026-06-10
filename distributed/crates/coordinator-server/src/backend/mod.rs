//! Backend abstraction layer.
//!
//! [`BackendService`] is the single trait that decouples the gRPC handlers
//! from the underlying implementation. Two implementations exist:
//!
//! - [`CoordinatorBackend`]: runs the coordinator in-process.
//! - [`MockBackend`]: in-memory, auto-progresses jobs; used for testing only.

pub mod coordinator;
pub mod mock;

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

use crate::errors::ApiResult;
use zisk_coordinator::WorkerSnapshot;

// Re-export domain types from coordinator-api so existing `use crate::backend::X` still works.
pub use zisk_coordinator_api::dto::*;

// Stream type aliases.

pub type JobEventStream = Pin<Box<dyn Stream<Item = ApiResult<DomainJobEvent>> + Send>>;
pub type InputChunkStream = Pin<Box<dyn Stream<Item = ApiResult<DomainInputChunk>> + Send>>;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveJobSnapshot {
    pub coordinator_id: String,
    pub job_id: Uuid,
    pub job_label: String,
    pub hash_id: String,
    pub program: String,
    pub state: String,
    pub phase: String,
    pub age_seconds: Option<u64>,
    pub phase_age_seconds: Option<u64>,
    pub update_age_seconds: u64,
    pub workers_count: usize,
}

// BackendService trait.

/// The single integration point between the gRPC handlers and the backend.
///
/// Swap [`MockBackend`] for [`CoordinatorBackend`] at startup; no handler
/// code changes required.
#[async_trait]
pub trait BackendService: Send + Sync + 'static {
    /// Register a guest program by ELF bytes. Idempotent: same ELF always
    /// returns the same `hash_id`.
    async fn register_guest_program(&self, elf: Vec<u8>) -> ApiResult<String>;

    /// Submit a new job. Returns the job UUID.
    async fn submit_job(&self, kind: DomainJobKind) -> ApiResult<SubmitJobResult>;

    /// Long-poll: block until the job reaches a terminal state or `timeout`
    /// elapses, then return the current state.
    async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> ApiResult<WaitResult>;

    /// Subscribe to state-transition events. The stream closes after the
    /// terminal event. Safe to call on an already-finished job.
    async fn watch_job(&self, job_id: Uuid) -> ApiResult<JobEventStream>;

    /// Feed stdin chunks to a job in `WaitingForInput` state.
    async fn push_job_input(&self, job_id: Uuid, chunks: InputChunkStream) -> ApiResult<()>;

    /// Feed hints chunks to a running job (gRPC push path).
    async fn push_job_hints_input(&self, job_id: Uuid, chunks: InputChunkStream) -> ApiResult<()>;

    /// Cancel a job. Blocks until the job reaches a terminal state, then
    /// returns `true` if the job was cancelled, or `false` if it was already
    /// in a terminal state when the request arrived.
    async fn cancel_job(&self, job_id: Uuid) -> ApiResult<bool>;
}

#[async_trait]
pub trait LiveStateProvider: Send + Sync + 'static {
    async fn current_live_job(
        &self,
        job_id: Option<Uuid>,
        program: Option<&str>,
    ) -> ApiResult<Option<LiveJobSnapshot>>;

    async fn live_workers(&self) -> ApiResult<Vec<WorkerSnapshot>>;
}
