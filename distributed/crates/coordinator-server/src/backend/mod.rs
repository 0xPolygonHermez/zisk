//! Backend abstraction layer.
//!
//! [`BackendService`] is the single trait that decouples the gRPC handlers
//! from the underlying implementation. Two implementations exist:
//!
//! - [`CoordinatorBackend`] — runs the coordinator in-process.
//! - [`MockBackend`] — in-memory, auto-progresses jobs; used for testing only.

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

pub type JobEventStream = Pin<Box<dyn Stream<Item = ApiResult<DomainJobEvent>> + Send>>;
pub type InputChunkStream = Pin<Box<dyn Stream<Item = ApiResult<DomainInputChunk>> + Send>>;

// ── BackendService trait ──────────────────────────────────────────────────────

/// The single integration point between the gRPC handlers and the backend.
///
/// Swap [`MockBackend`] for [`CoordinatorBackend`] at startup — no handler
/// code changes required.
#[async_trait]
pub trait BackendService: Send + Sync + 'static {
    /// Register a guest program by ELF bytes. Idempotent — same ELF always
    /// returns the same `hash_id`.
    async fn register_guest_program(&self, elf: Vec<u8>) -> ApiResult<String>;

    /// Submit a new job. Returns the job UUID.
    async fn submit_job(&self, kind: DomainJobKind) -> ApiResult<Uuid>;

    /// Long-poll: block until the job reaches a terminal state or `timeout`
    /// elapses, then return the current state.
    async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> ApiResult<WaitResult>;

    /// Subscribe to state-transition events. The stream closes after the
    /// terminal event. Safe to call on an already-finished job.
    async fn watch_job(&self, job_id: Uuid) -> ApiResult<JobEventStream>;

    /// Feed input chunks to a job in `WaitingForInput` state.
    async fn push_job_input(&self, job_id: Uuid, chunks: InputChunkStream) -> ApiResult<()>;

    /// Cancel a job. Blocks until the job reaches a terminal state, then
    /// returns `true` if the job was cancelled, or `false` if it was already
    /// in a terminal state when the request arrived.
    async fn cancel_job(&self, job_id: Uuid) -> ApiResult<bool>;
}
