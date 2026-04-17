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

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use prost_types::Timestamp;
use uuid::Uuid;

use crate::errors::GatewayResult;

use crate::proto::*;

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

impl From<DomainProofKind> for ProofKind {
    fn from(kind: DomainProofKind) -> Self {
        match kind {
            DomainProofKind::Stark => ProofKind::Stark,
            DomainProofKind::StarkMinimal => ProofKind::StarkMinimal,
            DomainProofKind::Plonk => ProofKind::Plonk,
        }
    }
}

impl From<zisk_common::ProofKind> for DomainProofKind {
    fn from(pk: zisk_common::ProofKind) -> Self {
        match pk {
            zisk_common::ProofKind::VadcopFinal => DomainProofKind::Stark,
            zisk_common::ProofKind::VadcopFinalMinimal => DomainProofKind::StarkMinimal,
            zisk_common::ProofKind::Plonk => DomainProofKind::Plonk,
        }
    }
}

impl TryFrom<i32> for DomainProofKind {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match ProofKind::try_from(value).unwrap_or(ProofKind::Unspecified) {
            ProofKind::Stark => Ok(DomainProofKind::Stark),
            ProofKind::StarkMinimal => Ok(DomainProofKind::StarkMinimal),
            ProofKind::Plonk => Ok(DomainProofKind::Plonk),
            _ => Err(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobPhase {
    Contributions,
    Prove,
    Aggregate,
}

impl From<DomainJobPhase> for JobPhase {
    fn from(phase: DomainJobPhase) -> Self {
        match phase {
            DomainJobPhase::Contributions => JobPhase::Contributions,
            DomainJobPhase::Prove => JobPhase::Prove,
            DomainJobPhase::Aggregate => JobPhase::Aggregate,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DomainInputChunk {
    pub data: Vec<u8>,
    pub is_last: bool,
}

impl From<InputChunk> for DomainInputChunk {
    fn from(val: InputChunk) -> Self {
        DomainInputChunk { data: val.data, is_last: val.is_last }
    }
}

impl From<DomainInputChunk> for InputChunk {
    fn from(chunk: DomainInputChunk) -> Self {
        InputChunk { data: chunk.data, is_last: chunk.is_last }
    }
}

#[derive(Debug, Clone)]
pub enum DomainInputKind {
    Inline(DomainInputChunk),
    StreamUri(String),
}

impl TryFrom<InputKind> for DomainInputKind {
    type Error = String;

    fn try_from(input: InputKind) -> Result<Self, Self::Error> {
        let kind = input.kind.ok_or_else(|| "input.kind must be set".to_string())?;
        match kind {
            input_kind::Kind::Inline(chunk) => Ok(DomainInputKind::Inline(chunk.into())),
            input_kind::Kind::StreamUri(uri) => Ok(DomainInputKind::StreamUri(uri)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DomainProof {
    pub proof_id: Uuid,
    pub hash_id: String,
    pub verification_key: Vec<u8>,
    pub proof_kind: DomainProofKind,
    pub data: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<DomainProof> for Proof {
    fn from(proof: DomainProof) -> Self {
        Proof {
            proof_id: proof.proof_id.to_string(),
            hash_id: proof.hash_id,
            verification_key: proof.verification_key,
            proof_kind: ProofKind::from(proof.proof_kind).into(),
            data: proof.data,
            public_inputs: proof.public_inputs,
            started_at: proof.started_at.map(datetime_to_ts),
            completed_at: proof.completed_at.map(datetime_to_ts),
        }
    }
}

impl TryFrom<Proof> for DomainProof {
    type Error = String;

    fn try_from(p: Proof) -> Result<Self, Self::Error> {
        Ok(DomainProof {
            proof_id: parse_uuid(&p.proof_id).map_err(|e| format!("invalid proof_id: {e}"))?,
            hash_id: p.hash_id,
            verification_key: p.verification_key,
            proof_kind: DomainProofKind::try_from(p.proof_kind)
                .map_err(|_| format!("invalid proof_kind {}", p.proof_kind))?,
            data: p.data,
            public_inputs: p.public_inputs,
            started_at: Some(p.started_at.map(ts_to_datetime).unwrap_or_else(chrono::Utc::now)),
            completed_at: Some(p.completed_at.map(ts_to_datetime).unwrap_or_else(chrono::Utc::now)),
        })
    }
}
fn datetime_to_ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp { seconds: dt.timestamp(), nanos: dt.timestamp_subsec_nanos() as i32 }
}

fn ts_to_datetime(ts: Timestamp) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.timestamp_opt(ts.seconds, ts.nanos as u32).single().unwrap_or_else(chrono::Utc::now)
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| anyhow::anyhow!("invalid UUID '{}': {e}", s))
}

// ── Job kinds ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum DomainJobKind {
    Setup(DomainSetupRequest),
    Prove(DomainProveRequest),
    Wrap(DomainWrapRequest),
    Execute(DomainExecuteRequest),
}

impl TryFrom<JobKind> for DomainJobKind {
    type Error = String;

    fn try_from(kind: JobKind) -> Result<Self, Self::Error> {
        let inner = kind.kind.ok_or_else(|| "job_kind.kind must be set".to_string())?;

        match inner {
            job_kind::Kind::Setup(r) => {
                Ok(DomainJobKind::Setup(DomainSetupRequest { hash_id: r.hash_id }))
            }
            job_kind::Kind::Prove(r) => {
                let input = r
                    .input
                    .ok_or_else(|| "input must be set".to_string())?
                    .try_into()
                    .map_err(|e: String| e)?;
                let proof_timeout = r.proof_timeout.map(ts_to_datetime);
                let proof_dest =
                    DomainProofKind::try_from(r.proof_dest).unwrap_or(DomainProofKind::Stark);
                Ok(DomainJobKind::Prove(DomainProveRequest {
                    hash_id: r.hash_id,
                    input,
                    proof_timeout,
                    proof_dest,
                }))
            }
            job_kind::Kind::Wrap(r) => {
                let proof = DomainProof::try_from(
                    r.proof.ok_or_else(|| "wrap.proof must be set".to_string())?,
                )
                .map_err(|e| format!("invalid wrap.proof: {e}"))?;
                let proof_dest = DomainProofKind::try_from(r.proof_dest)
                    .map_err(|_| "invalid proof_dest".to_string())?;
                let wrap_timeout = r.wrap_timeout.map(ts_to_datetime);
                Ok(DomainJobKind::Wrap(DomainWrapRequest { proof, proof_dest, wrap_timeout }))
            }
            job_kind::Kind::Execute(r) => {
                let input = r
                    .input
                    .ok_or_else(|| "input must be set".to_string())?
                    .try_into()
                    .map_err(|e: String| e)?;
                let execute_timeout = r.execute_timeout.map(ts_to_datetime);
                Ok(DomainJobKind::Execute(DomainExecuteRequest {
                    hash_id: r.hash_id,
                    input,
                    execute_timeout,
                }))
            }
        }
    }
}

impl From<DomainJobKind> for JobKind {
    fn from(domain: DomainJobKind) -> Self {
        use job_kind::Kind;
        let kind = match domain {
            DomainJobKind::Setup(r) => Kind::Setup(SetupRequest { hash_id: r.hash_id }),
            DomainJobKind::Prove(r) => {
                let input = match r.input {
                    DomainInputKind::Inline(chunk) => {
                        InputKind { kind: Some(input_kind::Kind::Inline(chunk.into())) }
                    }
                    DomainInputKind::StreamUri(uri) => {
                        InputKind { kind: Some(input_kind::Kind::StreamUri(uri)) }
                    }
                };

                Kind::Prove(ProveRequest {
                    hash_id: r.hash_id,
                    input: Some(input),
                    proof_timeout: r.proof_timeout.map(datetime_to_ts),
                    proof_dest: ProofKind::from(r.proof_dest).into(),
                })
            }
            DomainJobKind::Wrap(r) => Kind::Wrap(WrapRequest {
                proof: Some(r.proof.into()),
                proof_dest: ProofKind::from(r.proof_dest).into(),
                wrap_timeout: r.wrap_timeout.map(datetime_to_ts),
            }),
            DomainJobKind::Execute(r) => {
                let input = match r.input {
                    DomainInputKind::Inline(chunk) => {
                        InputKind { kind: Some(input_kind::Kind::Inline(chunk.into())) }
                    }
                    DomainInputKind::StreamUri(uri) => {
                        InputKind { kind: Some(input_kind::Kind::StreamUri(uri)) }
                    }
                };
                Kind::Execute(ExecuteRequest {
                    hash_id: r.hash_id,
                    input: Some(input),
                    execute_timeout: r.execute_timeout.map(datetime_to_ts),
                })
            }
        };
        JobKind { kind: Some(kind) }
    }
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
    pub proof_dest: DomainProofKind,
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

impl From<DomainExecutionStats> for ExecutionStats {
    fn from(stats: DomainExecutionStats) -> Self {
        ExecutionStats {
            steps: stats.steps,
            duration_nanos: stats.duration_nanos,
            cost_per_type: Some(CostPerType {
                main: stats.main_cost,
                opcode: stats.opcode_cost,
                memory: stats.memory_cost,
                precompile: stats.precompile_cost,
                tables: stats.tables_cost,
                other: stats.other_cost,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DomainJobKindResponse {
    Setup,
    Prove { proof: DomainProof, stats: DomainExecutionStats },
    Wrap(DomainProof),
    Execute { stats: DomainExecutionStats, public_outputs: Vec<u8> },
}

impl From<DomainJobKindResponse> for JobKindResponse {
    fn from(value: DomainJobKindResponse) -> Self {
        use job_kind_response::Kind;
        let kind = match value {
            DomainJobKindResponse::Setup => Kind::Setup(SetupResponse {}),
            DomainJobKindResponse::Prove { proof, stats } => {
                Kind::Prove(ProveResponse { proof: Some(proof.into()), stats: Some(stats.into()) })
            }
            DomainJobKindResponse::Wrap(proof) => {
                Kind::Wrap(WrapResponse { proof: Some(proof.into()) })
            }
            DomainJobKindResponse::Execute { stats, public_outputs } => {
                Kind::Execute(ExecuteResponse { stats: Some(stats.into()), public_outputs })
            }
        };
        JobKindResponse { kind: Some(kind) }
    }
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

impl From<&DomainJobStatus> for JobStatus {
    fn from(status: &DomainJobStatus) -> Self {
        let s = match status {
            DomainJobStatus::Queued => job_status::Status::Queued(JobStatusQueued {}),
            DomainJobStatus::Running(phase) => job_status::Status::Running(JobStatusRunning {
                phase: phase.as_ref().map(|p| JobPhase::from(p.clone()).into()),
            }),
            DomainJobStatus::WaitingForInput => {
                job_status::Status::WaitingForInput(JobStatusWaitingForInput {})
            }
            DomainJobStatus::Completed => job_status::Status::Completed(JobStatusCompleted {}),
            DomainJobStatus::Failed(f) => {
                job_status::Status::Failed(JobStatusFailed { failure: Some(f.into()) })
            }
            DomainJobStatus::Cancelled => job_status::Status::Cancelled(JobStatusCancelled {}),
        };
        JobStatus { status: Some(s) }
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

impl From<&DomainJobFailure> for JobFailure {
    fn from(failure: &DomainJobFailure) -> Self {
        use job_failure::Kind;
        let kind = match failure {
            DomainJobFailure::Timeout { phase, limit } => Kind::Timeout(JobFailureTimeout {
                phase: phase.as_ref().map(|p| JobPhase::from(p.clone()).into()),
                limit: Some(prost_types::Duration { seconds: limit.as_secs() as i64, nanos: 0 }),
            }),
            DomainJobFailure::Input { reason } => {
                Kind::Input(JobFailureInput { reason: reason.clone() })
            }
            DomainJobFailure::Execution { reason } => {
                Kind::Execution(JobFailureExecution { reason: reason.clone() })
            }
            DomainJobFailure::Internal { trace_id } => {
                Kind::Internal(JobFailureInternal { trace_id: trace_id.clone() })
            }
            DomainJobFailure::Cancelled => Kind::Cancelled(JobFailureCancelled {}),
        };
        JobFailure { kind: Some(kind) }
    }
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

impl From<DomainJobEvent> for JobEvent {
    fn from(event: DomainJobEvent) -> Self {
        use job_event::Event;
        let inner = match event {
            DomainJobEvent::Queued(e) => Event::Queued(JobEventQueued {
                job_id: e.job_id.to_string(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Started(e) => Event::Started(JobEventStarted {
                job_id: e.job_id.to_string(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Progress(e) => Event::Progress(JobEventProgress {
                job_id: e.job_id.to_string(),
                phase: JobPhase::from(e.phase).into(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::WaitingForInput(e) => Event::WaitingForInput(JobEventWaitingForInput {
                job_id: e.job_id.to_string(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Completed(e) => Event::Completed(JobEventCompleted {
                job_id: e.job_id.to_string(),
                result: Some(e.result.into()),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Cancelled(e) => Event::Cancelled(JobEventCancelled {
                job_id: e.job_id.to_string(),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
            DomainJobEvent::Failed(e) => Event::Failed(JobEventFailed {
                job_id: e.job_id.to_string(),
                failure: Some((&e.failure).into()),
                timestamp: Some(datetime_to_ts(e.timestamp)),
            }),
        };
        JobEvent { event: Some(inner) }
    }
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
