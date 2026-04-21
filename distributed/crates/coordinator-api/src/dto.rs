//! Domain types shared across coordinator and SDK.
//!
//! These types are the canonical representation of coordinator API concepts.

use std::time::Duration;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Convert a [`Duration`] into a future deadline.
///
/// Saturates to [`DateTime::<Utc>::MAX_UTC`] if `d` overflows `chrono::Duration`.
pub fn deadline_from_now(d: Duration) -> DateTime<Utc> {
    let chrono_dur = chrono::Duration::from_std(d).unwrap_or(chrono::Duration::MAX);
    Utc::now().checked_add_signed(chrono_dur).unwrap_or(DateTime::<Utc>::MAX_UTC)
}

pub struct RegisterGuestProgramRequestDto {
    /// The ELF bytes of the guest program to register.
    pub zisk_elf: Vec<u8>,
}

pub struct RegisterGuestProgramResponseDto {
    /// blake3 content hash of zisk_elf
    pub hash_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainProofKind {
    Stark,
    StarkMinimal,
    Plonk,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobPhase {
    Contributions,
    Prove,
    Aggregate,
}

#[derive(Debug, Clone)]
pub struct DomainInputChunk {
    pub data: Vec<u8>,
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
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum DomainJobKind {
    Setup(DomainSetupRequest),
    Prove(DomainProveRequest),
    Wrap(DomainWrapRequest),
    Execute(DomainExecuteRequest),
}

/// Optional compute capacity hint attached to a job request.
/// When absent the coordinator applies its configured defaults.
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

/// The terminal outcome of a job once it has reached a final state.
#[derive(Debug, Clone)]
pub enum TerminalStatus {
    Completed(DomainJobKindResponse),
    Failed(DomainJobFailure),
    Cancelled,
}

/// Returned by the coordinator `wait_job_result` long-poll.
#[derive(Debug)]
pub struct WaitResult {
    pub job_id: Uuid,
    pub job_status: DomainJobStatus,
    /// Present only when `job_status` is [`DomainJobStatus::Completed`].
    pub result: Option<DomainJobKindResponse>,
}
