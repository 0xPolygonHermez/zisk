//! Backend abstraction layer.
//!
//! [`BackendService`] is the single trait that decouples the gRPC handlers
//! from the underlying implementation. Two implementations exist:
//!
//! - [`MockBackend`] — in-memory, auto-progresses jobs; used for testing only.
//! - [`CoordinatorBackend`] — runs the coordinator in-process; the
//!   production deployment mode.

pub mod coordinator;
pub mod mock;

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use uuid::Uuid;

use crate::errors::GatewayResult;

// ── Domain types — independent of proto ──────────────────────────────────────
//
// These types mirror `book/developer/gateway_api.md` exactly. Proto ↔ domain
// conversions live in the `service/` layer.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainProofKind {
    Stark,
    StarkMinimal,
    Plonk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobPhase {
    Contributions,
    Prove,
    Aggregate,
}

#[derive(Debug, Clone)]
pub struct DomainInputChunk {
    pub data: Vec<u8>,
    pub is_last: bool,
}

#[derive(Debug, Clone)]
pub enum DomainInputKind {
    Inline(DomainInputChunk),
    StreamUri(String),
}

#[derive(Debug, Clone)]
pub struct DomainProof {
    pub proof_id: Uuid,
    pub hash_id: String,
    pub verification_key: Vec<u8>,
    pub proof_kind: DomainProofKind,
    pub data: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

// ── Job kinds ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum DomainJobKind {
    Setup(DomainSetupRequest),
    Prove(DomainProveRequest),
    Wrap(DomainWrapRequest),
    Execute(DomainExecuteRequest),
}

/// Optional compute capacity hint attached to a job request.
///
/// When absent the coordinator applies its configured defaults.
/// `requested` is clamped to available capacity; the job is rejected only if
/// available capacity falls below `minimum`.
#[derive(Debug, Clone)]
pub struct DomainComputeConstraints {
    pub requested: u32,
    pub minimum: u32,
}

#[derive(Debug, Clone)]
pub struct DomainSetupRequest {
    pub hash_id: String,
}

#[derive(Debug, Clone)]
pub struct DomainProveRequest {
    pub hash_id: String,
    pub input: DomainInputKind,
    pub proof_timeout: Option<DateTime<Utc>>,
    pub compute_constraints: Option<DomainComputeConstraints>,
}

#[derive(Debug, Clone)]
pub struct DomainWrapRequest {
    pub proof: DomainProof,
    pub proof_dest: DomainProofKind,
    pub wrap_timeout: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct DomainExecuteRequest {
    pub hash_id: String,
    pub input: DomainInputKind,
    pub execute_timeout: Option<DateTime<Utc>>,
    pub compute_constraints: Option<DomainComputeConstraints>,
}

// ── Job kind responses ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DomainExecutionStats {
    pub steps: u64,
    pub duration_nanos: u64,
    pub main_cost: u64,
    pub opcode_cost: u64,
    pub memory_cost: u64,
    pub precompile_cost: u64,
    pub tables_cost: u64,
    pub other_cost: u64,
}

#[derive(Debug, Clone)]
pub enum DomainJobKindResponse {
    Setup,
    Prove { proof: DomainProof, stats: DomainExecutionStats },
    Wrap(DomainProof),
    Execute { stats: DomainExecutionStats, public_outputs: Vec<u8> },
}

// ── Job status ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobStatus {
    Queued,
    Running(Option<DomainJobPhase>),
    WaitingForInput,
    Completed,
    Failed(DomainJobFailure),
    Cancelled,
}

impl DomainJobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobFailure {
    Timeout { phase: Option<DomainJobPhase>, limit: Duration },
    Input { reason: String },
    Execution { reason: String },
    Internal { trace_id: String },
    Cancelled,
}

// ── Job events ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum DomainJobEvent {
    Queued(DomainJobEventQueued),
    Started(DomainJobEventStarted),
    Progress(DomainJobEventProgress),
    WaitingForInput(DomainJobEventWaitingForInput),
    Completed(DomainJobEventCompleted),
    Cancelled(DomainJobEventCancelled),
    Failed(DomainJobEventFailed),
}

#[derive(Debug, Clone)]
pub struct DomainJobEventQueued {
    pub job_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DomainJobEventStarted {
    pub job_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DomainJobEventProgress {
    pub job_id: Uuid,
    pub phase: DomainJobPhase,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DomainJobEventWaitingForInput {
    pub job_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DomainJobEventCompleted {
    pub job_id: Uuid,
    pub result: DomainJobKindResponse,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DomainJobEventCancelled {
    pub job_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DomainJobEventFailed {
    pub job_id: Uuid,
    pub failure: DomainJobFailure,
    pub timestamp: DateTime<Utc>,
}

// ── WaitResult ────────────────────────────────────────────────────────────────

/// Returned by [`BackendService::wait_job_result`].
#[derive(Debug)]
pub struct WaitResult {
    pub job_id: Uuid,
    pub job_status: DomainJobStatus,
    /// Present only when `job_status` is [`DomainJobStatus::Completed`].
    pub result: Option<DomainJobKindResponse>,
}

// ── Stream type aliases ───────────────────────────────────────────────────────

pub type JobEventStream = Pin<Box<dyn Stream<Item = GatewayResult<DomainJobEvent>> + Send>>;
pub type InputChunkStream = Pin<Box<dyn Stream<Item = GatewayResult<DomainInputChunk>> + Send>>;

// ── BackendService trait ──────────────────────────────────────────────────────

/// The single integration point between the gRPC handlers and the backend.
///
/// Swap [`MockBackend`] for [`CoordinatorBackend`] at startup — no handler
/// code changes required.
#[async_trait]
pub trait BackendService: Send + Sync + 'static {
    /// Register a guest program by ELF bytes. Idempotent — same ELF always
    /// returns the same `hash_id`.
    async fn register_guest_program(&self, elf: Vec<u8>) -> GatewayResult<String>;

    /// Submit a new job. Returns the job UUID.
    async fn submit_job(&self, kind: DomainJobKind) -> GatewayResult<Uuid>;

    /// Long-poll: block until the job reaches a terminal state or `timeout`
    /// elapses, then return the current state.
    async fn wait_job_result(&self, job_id: Uuid, timeout: Duration) -> GatewayResult<WaitResult>;

    /// Subscribe to state-transition events. The stream closes after the
    /// terminal event. Safe to call on an already-finished job.
    async fn watch_job(&self, job_id: Uuid) -> GatewayResult<JobEventStream>;

    /// Feed input chunks to a job in `WaitingForInput` state.
    async fn push_job_input(&self, job_id: Uuid, chunks: InputChunkStream) -> GatewayResult<()>;

    /// Cancel a job. Blocks until the job reaches a terminal state, then
    /// returns `true` if the job was cancelled, or `false` if it was already
    /// in a terminal state when the request arrived.
    async fn cancel_job(&self, job_id: Uuid) -> GatewayResult<bool>;
}
