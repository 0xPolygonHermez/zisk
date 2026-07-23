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

/// Request to register a guest program by its ELF bytes.
pub struct RegisterGuestProgramRequestDto {
    /// The ELF bytes of the guest program to register.
    pub zisk_elf: Vec<u8>,
}

/// Response to guest-program registration.
pub struct RegisterGuestProgramResponseDto {
    /// blake3 content hash of zisk_elf
    pub hash_id: String,
}

/// A Circom normalization circuit body for an aggregation program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainNormalizeCircuit {
    /// The Circom source body.
    pub body: String,
}

/// Specification of an aggregation (recurser) program.
#[derive(Debug, Clone)]
pub struct DomainAggregationProgramSpec {
    /// Optional normalization circuit applied to inputs.
    pub normalize: Option<DomainNormalizeCircuit>,
    /// The Circom body that aggregates public values.
    pub aggregate_publics_body: String,
    /// Number of free inputs the aggregation consumes.
    pub n_free: u64,
    /// Publics slots the aggregation populates; the rest are generator-zero-filled.
    pub n_publics_agg: u64,
    /// Optional leaf allow-list: 4-limb program VKs baked into the recurser.
    /// Empty = VK-agnostic. Sent as VKs (not names) because the worker has no
    /// access to the client's guest ELFs; the worker recomputes `recurser_id`
    /// from these, so they must match the client's derivation.
    pub program_vks: Vec<[String; 4]>,
}

/// Request to register an aggregation program under a client-supplied id.
pub struct RegisterAggregationProgramRequestDto {
    /// SDK-computed content hash; the coordinator stores the spec under this key.
    pub recurser_id: String,
    /// The aggregation-program specification.
    pub spec: DomainAggregationProgramSpec,
}

/// Response to aggregation-program registration.
pub struct RegisterAggregationProgramResponseDto {
    /// The registered recurser id.
    pub recurser_id: String,
}

/// Result of `submit_job` — the coordinator-assigned job ID plus any
/// stream URIs allocated by the coordinator for auto-negotiated transports.
#[derive(Debug, Clone)]
pub struct SubmitJobResult {
    /// The assigned job id.
    pub job_id: Uuid,
}

/// The kind of proof requested or produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainProofKind {
    /// Full STARK (vadcop final) proof.
    Stark,
    /// Minimal STARK proof.
    StarkMinimal,
    /// PLONK/SNARK proof.
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

/// A phase of a running job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobPhase {
    /// Contribution-gathering phase.
    Contributions,
    /// Proof-generation phase.
    Prove,
    /// Recursion/aggregation phase.
    Recurse,
}

/// A chunk of streamed input data.
#[derive(Debug, Clone)]
pub struct DomainInputChunk {
    /// The chunk bytes.
    pub data: Vec<u8>,
}

/// How a job's input is supplied.
#[derive(Debug, Clone)]
pub enum DomainInputKind {
    /// Input supplied inline.
    Inline(DomainInputChunk),
    /// Input read from a stream URI.
    StreamUri(String),
}

/// A completed proof and its metadata.
#[derive(Debug, Clone)]
pub struct DomainProof {
    /// Unique proof id.
    pub proof_id: Uuid,
    /// Content hash of the proven program.
    pub hash_id: String,
    /// The program verification key.
    pub verification_key: Vec<u8>,
    /// The kind of proof.
    pub proof_kind: DomainProofKind,
    /// The serialized proof bytes.
    pub data: Vec<u8>,
    /// The proof's public inputs.
    pub public_inputs: Vec<u8>,
    /// When proving started, if known.
    pub started_at: Option<DateTime<Utc>>,
    /// When proving completed, if known.
    pub completed_at: Option<DateTime<Utc>>,
}

/// The kind of job being submitted.
#[derive(Debug, Clone)]
pub enum DomainJobKind {
    /// Program setup.
    Setup(DomainSetupRequest),
    /// Proof generation.
    Prove(DomainProveRequest),
    /// Proof wrapping.
    Wrap(DomainWrapRequest),
    /// Execute-only.
    Execute(DomainExecuteRequest),
    /// Aggregation-program setup.
    SetupAggregationProgram(DomainSetupAggregationProgramRequest),
    /// Proof aggregation.
    AggregateProofs(DomainAggregateProofsRequest),
}

/// Optional compute capacity hint attached to a job request.
/// When absent the coordinator applies its configured defaults.
#[derive(Debug, Clone)]
pub struct DomainComputeConstraints {
    /// Requested compute units.
    pub requested: u32,
    /// Minimum acceptable compute units.
    pub minimum: u32,
}

/// Request to set up a guest program on the cluster.
#[derive(Debug, Clone)]
pub struct DomainSetupRequest {
    /// Content hash of the program to set up.
    pub hash_id: String,
    /// Human-readable program name.
    pub program_name: String,
    /// Whether to enable the precompile hints path.
    pub with_hints: bool,
    /// Whether to set up for emulation only (no proving).
    pub emulator_only: bool,
}

/// Request to generate a proof.
#[derive(Debug, Clone)]
pub struct DomainProveRequest {
    /// Content hash of the program to prove.
    pub hash_id: String,
    /// The job input.
    pub input: DomainInputKind,
    /// Optional precompile hints input.
    pub hints: Option<DomainInputKind>,
    /// Optional deadline for the proof.
    pub proof_timeout: Option<DateTime<Utc>>,
    /// The desired output proof kind.
    pub proof_dest: DomainProofKind,
}

/// Request to wrap an existing proof into another kind.
#[derive(Debug, Clone)]
pub struct DomainWrapRequest {
    /// The proof to wrap.
    pub proof: DomainProof,
    /// The desired output proof kind.
    pub proof_dest: DomainProofKind,
    /// Optional deadline for the wrap.
    pub wrap_timeout: Option<DateTime<Utc>>,
}

/// Request to execute a program without proving.
#[derive(Debug, Clone)]
pub struct DomainExecuteRequest {
    /// Content hash of the program to execute.
    pub hash_id: String,
    /// The job input.
    pub input: DomainInputKind,
    /// Optional precompile hints input.
    pub hints: Option<DomainInputKind>,
    /// Optional deadline for the execution.
    pub execute_timeout: Option<DateTime<Utc>>,
}

/// Request to set up an aggregation program.
#[derive(Debug, Clone)]
pub struct DomainSetupAggregationProgramRequest {
    /// The recurser id to set up.
    pub recurser_id: String,
}

/// Request to aggregate two proofs through a recurser.
#[derive(Debug, Clone)]
pub struct DomainAggregateProofsRequest {
    /// The recurser id to aggregate through.
    pub recurser_id: String,
    /// bincode-serialized VadcopFinalProof.
    pub proof_a: Vec<u8>,
    /// bincode-serialized VadcopFinalProof (second input).
    pub proof_b: Vec<u8>,
    /// Free inputs for `proof_a`.
    pub free_inputs_a: Vec<u64>,
    /// Free inputs for `proof_b`.
    pub free_inputs_b: Vec<u64>,
    /// Optional aggregation root for the recurser.
    pub root_c_recurser_agg: Option<[u64; 4]>,
}

/// Execution statistics reported for a completed job.
#[derive(Debug, Clone, Default)]
pub struct DomainExecutionStats {
    /// Number of executed steps.
    pub steps: u64,
    /// Total execution duration, in nanoseconds.
    pub duration_nanos: u64,
    /// Cost attributed to the main state machine.
    pub main_cost: u64,
    /// Cost attributed to opcode execution.
    pub opcode_cost: u64,
    /// Cost attributed to memory operations.
    pub memory_cost: u64,
    /// Cost attributed to precompiles.
    pub precompile_cost: u64,
    /// Cost attributed to lookup tables.
    pub tables_cost: u64,
    /// Cost not attributed to any of the above categories.
    pub other_cost: u64,
    /// Per-phase executor timing breakdown.
    pub executor_time: DomainExecutorTime,
    /// Per-AIR instance plan (execute jobs only; empty otherwise).
    pub plan: Vec<DomainAirInstanceCount>,
}

/// Per-AIR planned instance count; the AIR name is derived from the ids by the consumer.
#[derive(Debug, Clone, Default)]
pub struct DomainAirInstanceCount {
    /// The AIR group id.
    pub airgroup_id: usize,
    /// The AIR id within the group.
    pub air_id: usize,
    /// Number of planned instances.
    pub count: u64,
}

/// Per-phase executor timing breakdown (milliseconds).
#[derive(Debug, Clone, Default)]
pub struct DomainExecutorTime {
    /// Total executor duration.
    pub total_duration: u64,
    /// Time spent in execution.
    pub execution_duration: u64,
    /// Time spent counting and planning.
    pub count_and_plan_duration: u64,
    /// Time spent counting and planning memory ops.
    pub count_and_plan_mo_duration: u64,
    /// ASM-specific timing, when the ASM backend was used.
    pub asm: Option<DomainAsmExecution>,
}

/// ASM emulator timing metrics.
#[derive(Debug, Clone)]
pub struct DomainAsmExecution {
    /// Wall-clock time, in seconds.
    pub time: f32,
    /// Effective execution rate, in MHz.
    pub mhz: f32,
}

/// The result payload for a completed job, by job kind.
#[derive(Debug, Clone)]
pub enum DomainJobKindResponse {
    /// Guest-program setup result.
    Setup {
        /// The program verification key.
        vk: Vec<u8>,
        /// The hash mode the key was generated with.
        hash_mode: String,
    },
    /// Proof-generation result.
    Prove {
        /// The generated proof.
        proof: DomainProof,
        /// Execution statistics.
        stats: DomainExecutionStats,
    },
    /// Proof-wrapping result.
    Wrap(DomainProof),
    /// Execute-only result.
    Execute {
        /// Execution statistics.
        stats: DomainExecutionStats,
        /// The program's public outputs.
        public_outputs: Vec<u8>,
    },
    /// Aggregation-program setup result.
    SetupAggregationProgram {
        /// The aggregation-program verification key.
        vk: Vec<u8>,
        /// The hash mode the key was generated with.
        hash_mode: String,
    },
    /// Proof-aggregation result.
    AggregateProofs(DomainProof),
}

/// The status of a job as tracked by the coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobStatus {
    /// Accepted, awaiting start.
    Queued,
    /// Running, optionally in a known phase.
    Running(Option<DomainJobPhase>),
    /// Paused awaiting streamed input.
    WaitingForInput,
    /// Finished successfully.
    Completed,
    /// Finished with a failure.
    Failed(DomainJobFailure),
    /// Cancelled.
    Cancelled,
}

impl DomainJobStatus {
    /// Whether this is a terminal status (completed, failed, or cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled)
    }
}

/// Why a job failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainJobFailure {
    /// A phase exceeded its time limit.
    Timeout {
        /// The phase that timed out, if known.
        phase: Option<DomainJobPhase>,
        /// The exceeded time limit.
        limit: Duration,
    },
    /// The supplied input was invalid.
    Input {
        /// Why the input was rejected.
        reason: String,
    },
    /// Execution failed.
    Execution {
        /// Why execution failed.
        reason: String,
    },
    /// An internal error (detail is logged; only a trace id is exposed).
    Internal {
        /// Trace id for correlating with server logs.
        trace_id: String,
    },
    /// The job was cancelled.
    Cancelled,
}

// The `Completed` variant is inherently large (it carries proofs + full execution
// stats); boxing it would add indirection to the common terminal path for no real gain.
/// An event streamed as a job progresses through its lifecycle.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum DomainJobEvent {
    /// The job was queued.
    Queued(DomainJobEventQueued),
    /// The job started.
    Started(DomainJobEventStarted),
    /// The job advanced to a new phase.
    Progress(DomainJobEventProgress),
    /// The job is awaiting streamed input.
    WaitingForInput(DomainJobEventWaitingForInput),
    /// The job completed successfully.
    Completed(DomainJobEventCompleted),
    /// The job was cancelled.
    Cancelled(DomainJobEventCancelled),
    /// The job failed.
    Failed(DomainJobEventFailed),
}

/// Payload for the `Queued` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventQueued {
    /// The job id.
    pub job_id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// Payload for the `Started` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventStarted {
    /// The job id.
    pub job_id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// Payload for the `Progress` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventProgress {
    /// The job id.
    pub job_id: Uuid,
    /// The phase the job advanced to.
    pub phase: DomainJobPhase,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// Payload for the `WaitingForInput` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventWaitingForInput {
    /// The job id.
    pub job_id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// Payload for the `Completed` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventCompleted {
    /// The job id.
    pub job_id: Uuid,
    /// The job result.
    pub result: DomainJobKindResponse,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// Payload for the `Cancelled` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventCancelled {
    /// The job id.
    pub job_id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// Payload for the `Failed` event.
#[derive(Debug, Clone)]
pub struct DomainJobEventFailed {
    /// The job id.
    pub job_id: Uuid,
    /// The failure cause.
    pub failure: DomainJobFailure,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
}

/// The terminal outcome of a job once it has reached a final state.
// `Completed` is inherently large (proofs + full execution stats); boxing it would add
// indirection to the common path for no real gain.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum TerminalStatus {
    /// Completed successfully, with its result.
    Completed(DomainJobKindResponse),
    /// Failed, with the cause.
    Failed(DomainJobFailure),
    /// Cancelled.
    Cancelled,
}

/// Returned by the coordinator `wait_job_result` long-poll.
#[derive(Debug)]
pub struct WaitResult {
    /// The job id.
    pub job_id: Uuid,
    /// The job's current status.
    pub job_status: DomainJobStatus,
    /// Present only when `job_status` is [`DomainJobStatus::Completed`].
    pub result: Option<DomainJobKindResponse>,
}
