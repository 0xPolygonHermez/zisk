//! Data Transfer Objects (DTOs) for Distributed Proving System
//!
//! This module defines the internal domain types used throughout the distributed proving system.
//! These DTOs serve as the canonical data structures for business logic, separate from external
//! representations like gRPC protobuf types or serialization formats.

use crate::{ComputeCapacity, DataId, JobId, WorkerId};
use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How a job's inputs are supplied to the cluster.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum InputsModeDto {
    /// No inputs are provided.
    InputsNone,
    /// Inputs are provided as a complete payload referenced by a URI.
    InputsPath(String),
    /// Inputs are provided directly as data: raw bytes exactly as they arrived
    /// on the API, handed to the transport unmodified.
    InputsData(Bytes),
    /// Inputs will be streamed from the given URI (QUIC, Unix socket).
    /// The coordinator reads from this URI and relays data to workers.
    InputsStream(String),
}

/// Hand-written so a multi-MB payload can never reach a log line — `Job`
/// derives `Debug`.
impl std::fmt::Debug for InputsModeDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputsModeDto::InputsNone => write!(f, "InputsNone"),
            InputsModeDto::InputsPath(path) => write!(f, "InputsPath({path})"),
            InputsModeDto::InputsData(data) => write!(f, "InputsData({} bytes)", data.len()),
            InputsModeDto::InputsStream(uri) => write!(f, "InputsStream({uri})"),
        }
    }
}

pub use zisk_common::AirInstanceCount;
pub use zisk_common::ProofKind;
pub use zisk_common::StatsCostPerType;

/// How a job's precompile hints are supplied to the cluster.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum HintsModeDto {
    /// No hints are provided.
    HintsNone,
    /// Hints are provided as a complete payload referenced by a URI.
    HintsPath(String),
    /// Hints are provided directly as data; see [`InputsModeDto::InputsData`].
    HintsData(Bytes),
    /// Hints will be streamed from the given URI endpoint.
    HintsStream(String),
}

/// See [`InputsModeDto`]'s.
impl std::fmt::Debug for HintsModeDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HintsModeDto::HintsNone => write!(f, "HintsNone"),
            HintsModeDto::HintsPath(path) => write!(f, "HintsPath({path})"),
            HintsModeDto::HintsData(data) => write!(f, "HintsData({} bytes)", data.len()),
            HintsModeDto::HintsStream(uri) => write!(f, "HintsStream({uri})"),
        }
    }
}

/// Request to launch a proof/execute job on the cluster.
pub struct LaunchProofRequestDto {
    /// Id of the input/hints data context.
    pub data_id: DataId,
    /// Content hash of the guest program.
    pub hash_id: String,
    /// Requested compute capacity, if any.
    pub compute_capacity: Option<u32>,
    /// Minimum acceptable compute capacity, if any.
    pub minimal_compute_capacity: Option<u32>,
    /// How inputs are supplied.
    pub inputs_mode: InputsModeDto,
    /// How hints are supplied.
    pub hints_mode: HintsModeDto,
    /// Simulated worker count, if running in simulation mode.
    pub simulated_node: Option<u32>,
    /// Arbitrary client metadata.
    pub metadata: Option<std::collections::BTreeMap<String, String>>,
    /// Whether to execute only (no proof).
    pub execution_only: bool,
    /// The kind of proof requested.
    pub proof_type: ProofKind,
}

/// Response to a launch request.
pub struct LaunchProofResponseDto {
    /// The assigned job id.
    pub job_id: JobId,
}

/// Request to wrap an existing proof into another kind.
pub struct LaunchWrapRequestDto {
    /// bincode-encoded `Proof` to wrap.
    pub proof_data: Vec<u8>,
    /// Target proof kind (a `ProofKind` value).
    pub proof_dest: i32,
}

/// Request from a worker to register with the coordinator.
pub struct WorkerRegisterRequestDto {
    /// The worker's id.
    pub worker_id: WorkerId,
    /// The worker's advertised compute capacity.
    pub compute_capacity: ComputeCapacity,
}

/// Request from a worker to reconnect after a disconnect.
pub struct WorkerReconnectRequestDto {
    /// The worker's id.
    pub worker_id: WorkerId,
    /// The worker's advertised compute capacity.
    pub compute_capacity: ComputeCapacity,
    /// The job the worker believes it was running, if any.
    pub last_known_job_id: Option<JobId>,
}

/// Reconciliation directive sent by the coordinator in the registration response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectionDirectiveDto {
    /// Worker has no stale state; proceed as idle.
    Idle,
    /// Worker's active job is still valid; keep computing.
    KeepComputing,
    /// Worker should cancel its stale local job and become idle.
    CancelStaleJob,
}

/// A message sent from the coordinator to a worker.
pub enum CoordinatorMessageDto {
    /// Liveness heartbeat.
    Heartbeat(HeartbeatDto),
    /// Request to shut down.
    Shutdown(ShutdownDto),
    /// Response to a worker registration.
    WorkerRegisterResponse(WorkerRegisterResponseDto),
    /// A task to execute.
    ExecuteTaskRequest(ExecuteTaskRequestDto),
    /// Notification that a job was cancelled.
    JobCancelled(JobCancelledDto),
    /// A chunk of streamed data.
    StreamData(StreamDataDto),
    /// Request to set up a guest program.
    SetupProgram(SetupProgramDto),
    /// A chunk of streamed job input.
    InputStreamData(InputStreamDataDto),
    /// Request to set up an aggregation program.
    SetupAggregationProgram(SetupAggregationProgramDto),
    /// Request to run a proof aggregation.
    RunAggregateProofs(RunAggregateProofsDto),
}

/// A Circom normalization circuit body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeCircuitDto {
    /// The Circom source body.
    pub body: String,
}

/// Specification of an aggregation (recurser) program.
#[derive(Debug, Clone)]
pub struct AggregationProgramSpecDto {
    /// Optional normalization circuit applied to inputs.
    pub normalize: Option<NormalizeCircuitDto>,
    /// The Circom body that aggregates public values.
    pub aggregate_publics_body: String,
    /// Number of free inputs the aggregation consumes.
    pub n_free: u64,
    /// Publics slots the aggregation populates; the rest are generator-zero-filled.
    pub n_publics_agg: u64,
    /// Optional leaf allow-list: 4-limb program VKs (empty = VK-agnostic).
    /// Order is significant; part of the `recurser_id` the worker recomputes.
    pub program_vks: Vec<[String; 4]>,
}

/// Coordinator → worker request to set up an aggregation program.
#[derive(Debug, Clone)]
pub struct SetupAggregationProgramDto {
    /// The job id.
    pub job_id: String,
    /// The recurser id to set up.
    pub recurser_id: String,
    /// The aggregation-program specification.
    pub spec: AggregationProgramSpecDto,
}

/// Coordinator → worker request to aggregate two proofs.
#[derive(Debug, Clone)]
pub struct RunAggregateProofsDto {
    /// The job id.
    pub job_id: String,
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

/// Worker → coordinator ack for an aggregation-program setup.
#[derive(Debug, Clone)]
pub struct SetupAggregationProgramAckDto {
    /// The job id.
    pub job_id: String,
    /// The acking worker.
    pub worker_id: WorkerId,
    /// The recurser id that was set up.
    pub recurser_id: String,
    /// Whether setup succeeded.
    pub success: bool,
    /// Error detail on failure.
    pub error_message: Option<String>,
    /// The aggregation-program verification key.
    pub vk: Vec<u8>,
    /// The hash mode the key was generated with.
    pub hash_mode: String,
}

/// Worker → coordinator ack for a proof aggregation.
#[derive(Debug, Clone)]
pub struct RunAggregateProofsAckDto {
    /// The job id.
    pub job_id: String,
    /// The acking worker.
    pub worker_id: WorkerId,
    /// Whether aggregation succeeded.
    pub success: bool,
    /// Error detail on failure.
    pub error_message: Option<String>,
    /// The aggregated proof bytes.
    pub proof: Vec<u8>,
}

/// A chunk of streamed job input pushed to a worker.
pub struct InputStreamDataDto {
    /// The target job.
    pub job_id: JobId,
    /// The input bytes. `Bytes` so the relay's per-worker clones are refcounts.
    pub payload: Bytes,
}

/// Coordinator → worker request to set up a guest program.
pub struct SetupProgramDto {
    /// The job id.
    pub job_id: String,
    /// The guest ELF bytes.
    pub elf_bytes: Vec<u8>,
    /// Content hash of the program.
    pub hash_id: String,
    /// Human-readable program name.
    pub program_name: String,
    /// Whether to enable the precompile hints path.
    pub with_hints: bool,
    /// Whether to set up for emulation only.
    pub emulator_only: bool,
}

/// The kind of a stream-data message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamMessageKind {
    /// Marks the beginning of a stream. No payload is expected.
    Start,
    /// Contains a chunk of stream data.
    Data,
    /// Marks the end of a stream. No payload is expected.
    End,
}

/// A stream-data message: start/data/end plus an optional payload.
#[derive(Debug, Clone)]
pub struct StreamDataDto {
    /// The target job.
    pub job_id: JobId,
    /// The kind of stream message.
    pub stream_type: StreamMessageKind,
    /// The payload, present for `Data` messages.
    pub stream_payload: Option<StreamPayloadDto>,
}

/// A sequenced chunk of stream data.
#[derive(Debug, Clone)]
pub struct StreamPayloadDto {
    /// Monotonic sequence number for reordering.
    pub sequence_number: u32,
    /// The chunk bytes; see [`InputStreamDataDto::payload`].
    pub payload: Bytes,
}

/// Liveness heartbeat from the coordinator.
pub struct HeartbeatDto {
    /// When the heartbeat was sent.
    pub timestamp: DateTime<Utc>,
}

/// Coordinator → worker shutdown request.
pub struct ShutdownDto {
    /// Human-readable shutdown reason.
    pub reason: String,
    /// Grace period, in seconds, before forced shutdown.
    pub grace_period_seconds: u32,
}

/// Response to a worker registration request.
pub struct WorkerRegisterResponseDto {
    /// The registered worker id.
    pub worker_id: WorkerId,
    /// Whether registration was accepted.
    pub accepted: bool,
    /// Human-readable status message.
    pub message: String,
    /// When the worker was registered.
    pub registered_at: DateTime<Utc>,
}

/// Notification that a job was cancelled.
pub struct JobCancelledDto {
    /// The cancelled job.
    pub job_id: JobId,
    /// Why it was cancelled.
    pub reason: String,
}

/// Coordinator → worker request to execute a phase task.
pub struct ExecuteTaskRequestDto {
    /// The target worker.
    pub worker_id: WorkerId,
    /// The job this task belongs to.
    pub job_id: JobId,
    /// The phase-specific parameters.
    pub params: ExecuteTaskRequestTypeDto,
    /// Job-level metadata propagated to the worker.
    pub metadata: Option<std::collections::BTreeMap<String, String>>,
}

/// Phase-specific parameters for an execute-task request.
pub enum ExecuteTaskRequestTypeDto {
    /// Contribution-phase parameters.
    ContributionParams(ContributionParamsDto),
    /// Prove-phase parameters.
    ProveParams(ProveParamsDto),
    /// Aggregation parameters.
    AggParams(AggParamsDto),
    /// Execute-only parameters (same shape as contribution).
    ExecutionParams(ContributionParamsDto),
    /// Wrap parameters.
    WrapParams(WrapParamsDto),
}

/// Parameters for a wrap task.
pub struct WrapParamsDto {
    /// bincode-encoded proof to wrap.
    pub proof_data: Vec<u8>,
    /// Target proof kind (a `ProofKind` value).
    pub proof_dest: i32,
}

/// Parameters for a contribution (or execute-only) task.
pub struct ContributionParamsDto {
    /// Content hash of the guest program.
    pub hash_id: String,
    /// Id of the data context.
    pub data_id: DataId,
    /// Where the input comes from.
    pub input_source: InputSourceDto,
    /// Where the hints come from.
    pub hints_source: HintsSourceDto,
    /// This worker's rank.
    pub rank_id: u32,
    /// Total workers on the job.
    pub total_workers: u32,
    /// This worker's compute-unit allocation.
    pub worker_allocation: Vec<u32>,
    /// Total compute units for the job.
    pub job_compute_units: ComputeCapacity,
}

/// Where a task's input is read from (borsh-encoded on the wire).
#[derive(Clone)]
pub enum InputSourceDto {
    /// Read from a file/URI path.
    InputPath(String),
    /// Inline input data. `Bytes` so per-worker clones don't copy the payload.
    InputData(Bytes),
    /// No input.
    InputNull,
}

/// See [`InputsModeDto`]'s.
impl std::fmt::Debug for InputSourceDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputSourceDto::InputPath(path) => write!(f, "InputPath({path})"),
            InputSourceDto::InputData(data) => write!(f, "InputData({} bytes)", data.len()),
            InputSourceDto::InputNull => write!(f, "InputNull"),
        }
    }
}

/// Hand-written because `Bytes` has no Borsh impl. Encodes as borsh does a
/// `Vec<u8>` variant — u8 variant index, u32-LE length, then the payload — which
/// is what MPI peers decode.
impl BorshSerialize for InputSourceDto {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Fully qualified: `serde::Serialize` is also in scope.
        match self {
            InputSourceDto::InputPath(path) => {
                BorshSerialize::serialize(&0u8, writer)?;
                BorshSerialize::serialize(path, writer)
            }
            InputSourceDto::InputData(data) => {
                BorshSerialize::serialize(&1u8, writer)?;
                BorshSerialize::serialize(&(data.len() as u32), writer)?;
                writer.write_all(data)
            }
            InputSourceDto::InputNull => BorshSerialize::serialize(&2u8, writer),
        }
    }
}

impl BorshDeserialize for InputSourceDto {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        match u8::deserialize_reader(reader)? {
            0 => Ok(InputSourceDto::InputPath(String::deserialize_reader(reader)?)),
            1 => {
                let len = u32::deserialize_reader(reader)? as usize;
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf)?;
                Ok(InputSourceDto::InputData(Bytes::from(buf)))
            }
            2 => Ok(InputSourceDto::InputNull),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid InputSourceDto variant {other}"),
            )),
        }
    }
}

/// Where a task's hints are read from (borsh-encoded on the wire).
#[derive(Clone)]
pub enum HintsSourceDto {
    /// Read from a file/URI path.
    HintsPath(String),
    /// Inline hint data; see [`InputSourceDto::InputData`].
    HintsData(Bytes),
    /// Streamed from a URI.
    HintsStream(String),
    /// No hints.
    HintsNull,
}

impl std::fmt::Debug for HintsSourceDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HintsSourceDto::HintsPath(path) => write!(f, "HintsPath({path})"),
            HintsSourceDto::HintsData(data) => write!(f, "HintsData({} bytes)", data.len()),
            HintsSourceDto::HintsStream(uri) => write!(f, "HintsStream({uri})"),
            HintsSourceDto::HintsNull => write!(f, "HintsNull"),
        }
    }
}

/// See [`InputSourceDto`]'s impls.
impl BorshSerialize for HintsSourceDto {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            HintsSourceDto::HintsPath(path) => {
                BorshSerialize::serialize(&0u8, writer)?;
                BorshSerialize::serialize(path, writer)
            }
            HintsSourceDto::HintsData(data) => {
                BorshSerialize::serialize(&1u8, writer)?;
                BorshSerialize::serialize(&(data.len() as u32), writer)?;
                writer.write_all(data)
            }
            HintsSourceDto::HintsStream(uri) => {
                BorshSerialize::serialize(&2u8, writer)?;
                BorshSerialize::serialize(uri, writer)
            }
            HintsSourceDto::HintsNull => BorshSerialize::serialize(&3u8, writer),
        }
    }
}

impl BorshDeserialize for HintsSourceDto {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        match u8::deserialize_reader(reader)? {
            0 => Ok(HintsSourceDto::HintsPath(String::deserialize_reader(reader)?)),
            1 => {
                let len = u32::deserialize_reader(reader)? as usize;
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf)?;
                Ok(HintsSourceDto::HintsData(Bytes::from(buf)))
            }
            2 => Ok(HintsSourceDto::HintsStream(String::deserialize_reader(reader)?)),
            3 => Ok(HintsSourceDto::HintsNull),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid HintsSourceDto variant {other}"),
            )),
        }
    }
}

/// Parameters for a prove task.
pub struct ProveParamsDto {
    /// The contribution challenges to prove against.
    pub challenges: Vec<ChallengesDto>,
}

/// Witness metadata reported by a worker.
#[derive(Clone)]
pub struct WitnessInfoDto {
    /// Witness computation time in milliseconds
    pub witness_time: f32,
    /// The program's public values.
    pub publics: Vec<u64>,
    /// Proof values produced alongside the witness.
    pub proof_values: Vec<u64>,
    /// Human-readable summary of the witness.
    pub summary_info: String,
    /// Number of AIR instances.
    pub total_instances: u64,
}

/// Executor timing breakdown reported by a worker.
#[derive(Clone)]
pub struct ZiskExecutorTimeDto {
    /// Total duration in milliseconds
    pub total_duration: f32,
    /// Execution duration in milliseconds
    pub execution_duration: f32,
    /// Count and plan duration in milliseconds
    pub count_and_plan_duration: f32,
    /// Count and plan memory operations duration in milliseconds
    pub count_and_plan_mo_duration: f32,
    /// ASM execution info (time in seconds)
    pub asm_execution_duration: Option<AsmExecutionInfoDto>,
    /// Time when task was received by worker (milliseconds since UNIX epoch, f64 for precision)
    pub task_received_time: f64,
}

/// ASM emulator timing metrics.
#[derive(Clone)]
pub struct AsmExecutionInfoDto {
    /// Wall-clock time, in seconds.
    pub time: f32,
    /// Effective execution rate, in MHz.
    pub mhz: f32,
}

/// A single worker's contribution challenge for an airgroup.
#[derive(Clone)]
pub struct ChallengesDto {
    /// Index of the producing worker.
    pub worker_index: u32,
    /// The airgroup this challenge belongs to.
    pub airgroup_id: u32,
    /// The challenge field-element values.
    pub challenge: Vec<u64>,
}

/// Result payload of an execute-only task.
pub struct ExecutionResultDataDto {
    /// Number of AIR instances.
    pub instances: u64,
    /// Steps executed.
    pub executed_steps: u64,
    /// Executor timing.
    pub zisk_executor_time: ZiskExecutorTimeDto,
    /// The program's public values.
    pub publics: Vec<u64>,
    /// Per-type cost breakdown.
    pub cost_per_type: StatsCostPerType,
    /// Per-AIR instance plan.
    pub plan: Vec<AirInstanceCount>,
}

/// Parameters for an aggregation task.
pub struct AggParamsDto {
    /// The partial proofs to aggregate.
    pub agg_proofs: Vec<ProofStarkDto>,
    /// Whether this is the last proof in the round.
    pub last_proof: bool,
    /// Whether this produces the final job proof.
    pub final_proof: bool,
    /// The kind of proof being produced.
    pub proof_type: ProofKind,
}

/// A worker's partial STARK proof for an airgroup.
pub struct ProofStarkDto {
    /// Index of the producing worker.
    pub worker_idx: u32,
    /// The airgroup this proof belongs to.
    pub airgroup_id: u64,
    /// The proof field-element values.
    pub values: Vec<u64>,
}

/// The final aggregated proof and its stats.
pub struct FinalProofDto {
    /// The serialized proof bytes.
    pub proof_data: Vec<u8>,
    /// Total steps executed for the job.
    pub executed_steps: u64,
    /// Number of AIR instances.
    pub instances: u64,
}

/// Worker → coordinator response to an execute-task request.
pub struct ExecuteTaskResponseDto {
    /// The job id.
    pub job_id: JobId,
    /// The responding worker.
    pub worker_id: WorkerId,
    /// Whether the task succeeded.
    pub success: bool,
    /// Error detail on failure.
    pub error_message: Option<String>,
    /// `None` is only valid on failure responses (e.g. dispatch failure before
    /// any computation). On success the variant must match the expected phase.
    pub result_data: Option<ExecuteTaskResponseResultDataDto>,
    /// Whether the worker is currently in recovery.
    pub worker_in_recovery: bool,
}

/// Result payload of a contribution task.
pub struct ContributionsResultDataDto {
    /// The contribution challenges.
    pub challenges: Vec<ChallengesDto>,
    /// Witness metadata.
    pub witness_info: WitnessInfoDto,
    /// Executor timing.
    pub zisk_executor_time: ZiskExecutorTimeDto,
    /// Per-type cost breakdown.
    pub cost_per_type: StatsCostPerType,
}

/// The phase-specific result payload of an execute-task response.
pub enum ExecuteTaskResponseResultDataDto {
    /// Execute-only result.
    Execution(ExecutionResultDataDto),
    /// Contribution-phase result.
    Challenges(ContributionsResultDataDto),
    /// Partial proofs from the prove phase.
    Proofs(Vec<ProofStarkDto>),
    /// The final aggregated proof.
    FinalProof(FinalProofDto),
    /// A wrapped proof.
    WrapResult(WrapResultDto),
}

/// Result payload of a wrap task.
pub struct WrapResultDto {
    /// The wrapped proof bytes.
    pub proof_data: Vec<u8>,
}

/// Worker → coordinator heartbeat acknowledgement.
pub struct HeartbeatAckDto {
    /// The acking worker.
    pub worker_id: WorkerId,
}

/// Worker → coordinator ack for a guest-program setup.
pub struct SetupProgramAckDto {
    /// The job id.
    pub job_id: String,
    /// The acking worker.
    pub worker_id: WorkerId,
    /// Content hash of the program that was set up.
    pub hash_id: String,
    /// Whether setup succeeded.
    pub success: bool,
    /// Error detail on failure.
    pub error_message: Option<String>,
    /// The program verification key.
    pub vk: Vec<u8>,
    /// The hash mode the key was generated with.
    pub hash_mode: String,
}

/// Worker → coordinator error report.
pub struct WorkerErrorDto {
    /// The reporting worker.
    pub worker_id: WorkerId,
    /// The affected job.
    pub job_id: JobId,
    /// The error detail.
    pub error_message: String,
}

/// Error information for webhook notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookErrorDto {
    /// Machine-readable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

/// Webhook payload for job completion notifications
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayloadDto {
    /// The job id.
    pub job_id: String,
    /// Whether the job succeeded.
    pub success: bool,
    /// Total job duration, in milliseconds.
    pub duration_ms: u64,
    /// Steps executed, if known.
    pub executed_steps: Option<u64>,
    /// RFC 3339 timestamp of the notification.
    pub timestamp: String,
    /// Error details, present on failure.
    pub error: Option<WebhookErrorDto>,
    /// The proof bytes, when included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_data: Option<Vec<u8>>,
}

impl WebhookPayloadDto {
    /// Creates a successful webhook payload
    pub fn success(
        job_id: String,
        duration_ms: u64,
        executed_steps: Option<u64>,
        proof_data: Option<Vec<u8>>,
    ) -> Self {
        Self {
            job_id,
            success: true,
            duration_ms,
            executed_steps,
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: None,
            proof_data,
        }
    }

    /// Creates a failed webhook payload with error details
    pub fn failure(job_id: String, duration_ms: u64, error: WebhookErrorDto) -> Self {
        Self {
            job_id,
            success: false,
            duration_ms,
            executed_steps: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: Some(error),
            proof_data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout is u8 variant index, u32-LE length, then raw bytes. It travels
    /// over MPI, so a slip corrupts peer ranks rather than failing to compile.
    #[test]
    fn input_source_borsh_layout_matches_derived_vec_encoding() {
        let encoded = borsh::to_vec(&InputSourceDto::InputData(Bytes::from_static(&[7, 8, 9])))
            .expect("serialize");
        assert_eq!(encoded, vec![1, 3, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn hints_source_borsh_layout_matches_derived_vec_encoding() {
        let encoded =
            borsh::to_vec(&HintsSourceDto::HintsData(Bytes::from_static(&[1, 2]))).expect("ser");
        assert_eq!(encoded, vec![1, 2, 0, 0, 0, 1, 2]);
    }

    #[test]
    fn input_source_round_trips_every_variant() {
        let cases = [
            InputSourceDto::InputPath("/tmp/in".to_string()),
            InputSourceDto::InputData(Bytes::from_static(&[0, 255, 128])),
            InputSourceDto::InputData(Bytes::new()),
            InputSourceDto::InputNull,
        ];
        for case in cases {
            let bytes = borsh::to_vec(&case).expect("serialize");
            let back: InputSourceDto = borsh::from_slice(&bytes).expect("deserialize");
            assert_eq!(format!("{case:?}"), format!("{back:?}"));
            if let (InputSourceDto::InputData(a), InputSourceDto::InputData(b)) = (&case, &back) {
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn hints_source_round_trips_every_variant() {
        let cases = [
            HintsSourceDto::HintsPath("/tmp/h".to_string()),
            HintsSourceDto::HintsData(Bytes::from_static(&[3, 4, 5])),
            HintsSourceDto::HintsStream("quic://127.0.0.1:9".to_string()),
            HintsSourceDto::HintsNull,
        ];
        for case in cases {
            let bytes = borsh::to_vec(&case).expect("serialize");
            let back: HintsSourceDto = borsh::from_slice(&bytes).expect("deserialize");
            assert_eq!(format!("{case:?}"), format!("{back:?}"));
            if let (HintsSourceDto::HintsData(a), HintsSourceDto::HintsData(b)) = (&case, &back) {
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn payload_debug_does_not_dump_bytes() {
        let big = InputsModeDto::InputsData(Bytes::from(vec![0u8; 4096]));
        assert_eq!(format!("{big:?}"), "InputsData(4096 bytes)");
        let src = InputSourceDto::InputData(Bytes::from(vec![0u8; 4096]));
        assert_eq!(format!("{src:?}"), "InputData(4096 bytes)");
    }

    #[test]
    fn invalid_variant_index_is_an_error_not_a_panic() {
        assert!(borsh::from_slice::<InputSourceDto>(&[9]).is_err());
        assert!(borsh::from_slice::<HintsSourceDto>(&[9]).is_err());
    }
}
