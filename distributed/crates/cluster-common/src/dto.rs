//! Data Transfer Objects (DTOs) for Distributed Proving System
//!
//! This module defines the internal domain types used throughout the distributed proving system.
//! These DTOs serve as the canonical data structures for business logic, separate from external
//! representations like gRPC protobuf types or serialization formats.

use crate::{ComputeCapacity, DataId, JobId, WorkerId};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputsModeDto {
    // No inputs are provided
    InputsNone,
    /// Inputs are provided as a complete payload referenced by a URI.
    InputsPath(String),
    /// Inputs are provided directly as data.
    InputsData(String),
    /// Inputs will be streamed from the given URI (QUIC, Unix socket).
    /// The coordinator reads from this URI and relays data to workers.
    InputsStream(String),
}

pub use zisk_common::ProofKind;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HintsModeDto {
    /// No hints are provided.
    HintsNone,
    /// Hints are provided as a complete payload referenced by a URI.
    HintsPath(String),
    /// Hints are provided directly as data (hex-encoded).
    HintsData(String),
    /// Hints will be streamed from the given URI endpoint.
    HintsStream(String),
}

pub struct LaunchProofRequestDto {
    pub data_id: DataId,
    pub hash_id: String,
    pub compute_capacity: Option<u32>,
    pub minimal_compute_capacity: Option<u32>,
    pub inputs_mode: InputsModeDto,
    pub hints_mode: HintsModeDto,
    pub simulated_node: Option<u32>,
    pub metadata: std::collections::BTreeMap<String, String>,
    pub execution_only: bool,
    pub proof_type: ProofKind,
}

pub struct LaunchProofResponseDto {
    pub job_id: JobId,
}

pub struct LaunchWrapRequestDto {
    pub proof_data: Vec<u8>, // bincode-encoded Proof
    pub proof_dest: i32,     // ProofKind value
}

pub struct WorkerRegisterRequestDto {
    pub worker_id: WorkerId,
    pub compute_capacity: ComputeCapacity,
}

pub struct WorkerReconnectRequestDto {
    pub worker_id: WorkerId,
    pub compute_capacity: ComputeCapacity,
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

pub enum CoordinatorMessageDto {
    Heartbeat(HeartbeatDto),
    Shutdown(ShutdownDto),
    WorkerRegisterResponse(WorkerRegisterResponseDto),
    ExecuteTaskRequest(ExecuteTaskRequestDto),
    JobCancelled(JobCancelledDto),
    StreamData(StreamDataDto),
    SetupProgram(SetupProgramDto),
    InputStreamData(InputStreamDataDto),
    SetupRecurserAggregator(SetupRecurserAggregatorDto),
    RunRecurserAggregator(RunRecurserAggregatorDto),
}

/// 4-limb Goldilocks verification key, decimal-encoded.
pub type AggregatorProgramVk = [String; 4];

#[derive(Debug, Clone)]
pub struct AggregatorSpecDto {
    pub program_vks: Vec<AggregatorProgramVk>,
    pub n_private_inputs: u64,
    pub prepare_publics_body: String,
    pub check_publics_body: String,
    pub aggregate_publics_body: String,
}

#[derive(Debug, Clone)]
pub struct SetupRecurserAggregatorDto {
    pub job_id: String,
    pub recurser_id: String,
    pub spec: AggregatorSpecDto,
}

#[derive(Debug, Clone)]
pub struct RunRecurserAggregatorDto {
    pub job_id: String,
    pub recurser_id: String,
    /// bincode-serialized VadcopFinalProof.
    pub proof_a: Vec<u8>,
    pub proof_b: Vec<u8>,
    pub private_inputs: Vec<u64>,
    pub root_c_recurser_agg: Option<[u64; 4]>,
}

#[derive(Debug, Clone)]
pub struct SetupRecurserAggregatorAckDto {
    pub job_id: String,
    pub worker_id: WorkerId,
    pub recurser_id: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub vk: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RunRecurserAggregatorAckDto {
    pub job_id: String,
    pub worker_id: WorkerId,
    pub success: bool,
    pub error_message: Option<String>,
    pub proof: Vec<u8>,
}

pub struct InputStreamDataDto {
    pub job_id: JobId,
    pub payload: Vec<u8>,
}

pub struct SetupProgramDto {
    pub job_id: String,
    pub elf_bytes: Vec<u8>,
    pub hash_id: String,
    pub program_name: String,
    pub with_hints: bool,
    pub emulator_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StreamMessageKind {
    /// Marks the beginning of a stream. No payload is expected.
    Start,
    /// Contains a chunk of stream data.
    Data,
    /// Marks the end of a stream. No payload is expected.
    End,
}

#[derive(Debug, Clone)]
pub struct StreamDataDto {
    pub job_id: JobId,
    pub stream_type: StreamMessageKind,
    pub stream_payload: Option<StreamPayloadDto>,
}

#[derive(Debug, Clone)]
pub struct StreamPayloadDto {
    pub sequence_number: u32,
    pub payload: Vec<u8>,
}

pub struct HeartbeatDto {
    pub timestamp: DateTime<Utc>,
}

pub struct ShutdownDto {
    pub reason: String,
    pub grace_period_seconds: u32,
}

pub struct WorkerRegisterResponseDto {
    pub worker_id: WorkerId,
    pub accepted: bool,
    pub message: String,
    pub registered_at: DateTime<Utc>,
}

pub struct JobCancelledDto {
    pub job_id: JobId,
    pub reason: String,
}

pub struct ExecuteTaskRequestDto {
    pub worker_id: WorkerId,
    pub job_id: JobId,
    pub params: ExecuteTaskRequestTypeDto,
}

pub enum ExecuteTaskRequestTypeDto {
    ContributionParams(ContributionParamsDto),
    ProveParams(ProveParamsDto),
    AggParams(AggParamsDto),
    ExecutionParams(ContributionParamsDto),
    WrapParams(WrapParamsDto),
}

pub struct WrapParamsDto {
    pub proof_data: Vec<u8>,
    pub proof_dest: i32,
}

pub struct ContributionParamsDto {
    pub hash_id: String,
    pub data_id: DataId,
    pub input_source: InputSourceDto,
    pub hints_source: HintsSourceDto,
    pub rank_id: u32,
    pub total_workers: u32,
    pub worker_allocation: Vec<u32>,
    pub job_compute_units: ComputeCapacity,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum InputSourceDto {
    InputPath(String),
    InputData(Vec<u8>),
    InputNull,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum HintsSourceDto {
    HintsPath(String),
    HintsData(Vec<u8>),
    HintsStream(String),
    HintsNull,
}

pub struct ProveParamsDto {
    pub challenges: Vec<ChallengesDto>,
}

#[derive(Clone)]
pub struct WitnessInfoDto {
    /// Witness computation time in milliseconds
    pub witness_time: f32,
    pub publics: Vec<u64>,
    pub proof_values: Vec<u64>,
    pub summary_info: String,
    pub total_instances: u64,
}

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
    /// ASM execution info (time in milliseconds)
    pub asm_execution_duration: Option<AsmExecutionInfoDto>,
    /// Time when task was received by worker (milliseconds since UNIX epoch, f64 for precision)
    pub task_received_time: f64,
}

#[derive(Clone)]
pub struct AsmExecutionInfoDto {
    pub time: f32,
    pub mhz: f32,
}

#[derive(Clone)]
pub struct ChallengesDto {
    pub worker_index: u32,
    pub airgroup_id: u32,
    pub challenge: Vec<u64>,
}

pub struct ExecutionResultDataDto {
    pub instances: u64,
    pub executed_steps: u64,
    pub zisk_executor_time: ZiskExecutorTimeDto,
    pub publics: Vec<u64>,
}

pub struct AggParamsDto {
    pub agg_proofs: Vec<ProofStarkDto>,
    pub last_proof: bool,
    pub final_proof: bool,
    pub proof_type: ProofKind,
}

pub struct ProofStarkDto {
    pub worker_idx: u32,
    pub airgroup_id: u64,
    pub values: Vec<u64>,
}

pub struct FinalProofDto {
    pub proof_data: Vec<u8>,
    pub executed_steps: u64,
    pub instances: u64,
}

pub struct ExecuteTaskResponseDto {
    pub job_id: JobId,
    pub worker_id: WorkerId,
    pub success: bool,
    pub error_message: Option<String>,
    /// `None` is only valid on failure responses (e.g. dispatch failure before
    /// any computation). On success the variant must match the expected phase.
    pub result_data: Option<ExecuteTaskResponseResultDataDto>,
    pub worker_in_recovery: bool,
}

pub struct ContributionsResultDataDto {
    pub challenges: Vec<ChallengesDto>,
    pub witness_info: WitnessInfoDto,
    pub zisk_executor_time: ZiskExecutorTimeDto,
}

pub enum ExecuteTaskResponseResultDataDto {
    Execution(ExecutionResultDataDto),
    Challenges(ContributionsResultDataDto),
    Proofs(Vec<ProofStarkDto>),
    FinalProof(FinalProofDto),
    WrapResult(WrapResultDto),
}

pub struct WrapResultDto {
    pub proof_data: Vec<u8>,
}

pub struct HeartbeatAckDto {
    pub worker_id: WorkerId,
}

pub struct SetupProgramAckDto {
    pub job_id: String,
    pub worker_id: WorkerId,
    pub hash_id: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub vk: Vec<u8>,
}

pub struct WorkerErrorDto {
    pub worker_id: WorkerId,
    pub job_id: JobId,
    pub error_message: String,
}

/// Error information for webhook notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookErrorDto {
    pub code: String,
    pub message: String,
}

/// Webhook payload for job completion notifications
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayloadDto {
    pub job_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub executed_steps: Option<u64>,
    pub timestamp: String,
    pub error: Option<WebhookErrorDto>,
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
