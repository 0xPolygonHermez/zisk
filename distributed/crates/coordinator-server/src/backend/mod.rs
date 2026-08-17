//! Backend abstraction layer.
//!
//! [`BackendService`](crate::backend::BackendService) is the single trait that decouples the gRPC handlers
//! from the underlying implementation. Two implementations exist:
//!
//! - [`CoordinatorBackend`](crate::backend::coordinator::CoordinatorBackend) — runs the coordinator in-process.
//! - [`MockBackend`](crate::backend::mock::MockBackend) — in-memory, auto-progresses jobs; used for testing only.

pub mod coordinator;
pub mod mock;

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

use crate::errors::ApiResult;

// Re-export domain types from coordinator-api so existing `use crate::backend::X` still works.
pub use zisk_coordinator_api::dto::*;

// ── Stream type aliases ───────────────────────────────────────────────────────

/// Stream of job lifecycle events produced by `watch_job`.
pub type JobEventStream = Pin<Box<dyn Stream<Item = ApiResult<DomainJobEvent>> + Send>>;
/// Stream of input/hint chunks consumed by the `push_job_*_input` operations.
pub type InputChunkStream = Pin<Box<dyn Stream<Item = ApiResult<DomainInputChunk>> + Send>>;

// ── BackendService trait ──────────────────────────────────────────────────────

/// The single integration point between the gRPC handlers and the backend.
///
/// Swap [`MockBackend`](crate::backend::mock::MockBackend) for [`CoordinatorBackend`](crate::backend::coordinator::CoordinatorBackend) at startup — no handler
/// code changes required.
#[async_trait]
pub trait BackendService: Send + Sync + 'static {
    /// Register a guest program by ELF bytes. Idempotent — same ELF always
    /// returns the same `hash_id`.
    async fn register_guest_program(&self, elf: Vec<u8>) -> ApiResult<String>;

    /// Register a recurser spec under the SDK-supplied `recurser_id`.
    /// Idempotent — re-registering the same id is a no-op. Returns the id (echo).
    async fn register_aggregation_program(
        &self,
        recurser_id: String,
        spec: DomainAggregationProgramSpec,
    ) -> ApiResult<String>;

    /// Submit a new job. Returns the job UUID.
    ///
    /// Submit a base job. The base API carries no caller metadata.
    async fn submit_job(&self, kind: DomainJobKind) -> ApiResult<SubmitJobResult> {
        self.submit_job_with_metadata(kind, None).await
    }

    /// Submit a job with caller-defined key/value metadata (the extended API).
    async fn submit_job_ext(
        &self,
        kind: DomainJobKind,
        metadata: std::collections::BTreeMap<String, String>,
    ) -> ApiResult<SubmitJobResult> {
        self.submit_job_with_metadata(kind, Some(metadata)).await
    }

    /// Implementation seam shared by [`submit_job`](Self::submit_job) and
    /// [`submit_job_ext`](Self::submit_job_ext); backends implement this one.
    async fn submit_job_with_metadata(
        &self,
        kind: DomainJobKind,
        metadata: Option<std::collections::BTreeMap<String, String>>,
    ) -> ApiResult<SubmitJobResult>;

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
