use crate::{BlockId, ComputeCapacity, JobId, JobPhase, JobState, ProverId, ProverState};
use chrono::{DateTime, Utc};

pub struct StatusInfoDto {
    pub service_name: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub start_time: DateTime<Utc>,
    pub metrics: MetricsDto,
}

impl StatusInfoDto {
    pub fn new(
        service_name: String,
        version: String,
        uptime_seconds: u64,
        start_time: DateTime<Utc>,
        metrics: MetricsDto,
    ) -> Self {
        Self { service_name, version, uptime_seconds, start_time, metrics }
    }
}

pub struct JobsListDto {
    pub jobs: Vec<JobStatusDto>,
}

pub struct JobStatusDto {
    pub job_id: JobId,
    pub block_id: BlockId,
    pub status: JobState,
    pub phase: Option<JobPhase>,
    pub assigned_provers: Vec<ProverId>,
    pub start_time: u64,
    pub duration_ms: u64,
}

pub struct ProversListDto {
    pub provers: Vec<ProverInfoDto>,
}

pub struct ProverInfoDto {
    pub prover_id: ProverId,
    pub state: ProverState,
    pub compute_capacity: ComputeCapacity,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

pub struct SystemStatusDto {
    pub total_provers: u32,
    pub compute_capacity: ComputeCapacity,
    pub idle_provers: u32,
    pub busy_provers: u32,
    pub active_jobs: u32,
}

pub struct LaunchProofRequestDto {
    pub block_id: BlockId,
    pub compute_units: u32,
    pub input_path: String,
}

pub struct LaunchProofResponseDto {
    pub job_id: JobId,
}

pub struct MetricsDto {
    pub active_connections: u32,
}

pub struct ProverRegisterRequestDto {
    pub prover_id: ProverId,
    pub compute_capacity: ComputeCapacity,
}

pub struct ProverReconnectRequestDto {
    pub prover_id: ProverId,
    pub compute_capacity: ComputeCapacity,
}

pub enum CoordinatorMessageDto {
    Heartbeat(HeartbeatDto),
    Shutdown(ShutdownDto),
    ProverRegisterResponse(ProverRegisterResponseDto),
    ExecuteTaskRequest(ExecuteTaskRequestDto),
    JobCancelled(JobCancelledDto),
}

pub struct HeartbeatDto {
    pub timestamp: DateTime<Utc>,
}

pub struct ShutdownDto {
    pub reason: String,
    pub grace_period_seconds: u32,
}

pub struct ProverRegisterResponseDto {
    pub prover_id: ProverId,
    pub accepted: bool,
    pub message: String,
    pub registered_at: DateTime<Utc>,
}

pub struct JobCancelledDto {
    pub job_id: JobId,
    pub reason: String,
}

pub struct ExecuteTaskRequestDto {
    pub prover_id: ProverId,
    pub job_id: JobId,
    pub params: ExecuteTaskRequestTypeDto,
}

pub enum ExecuteTaskRequestTypeDto {
    ContributionParams(ContributionParamsDto),
    ProveParams(ProveParamsDto),
    AggParams(AggParamsDto),
}

pub struct ContributionParamsDto {
    pub block_id: BlockId,
    pub input_path: String,
    pub rank_id: u32,
    pub total_provers: u32,
    pub prover_allocation: Vec<u32>,
    pub job_compute_units: ComputeCapacity,
}

pub struct ProveParamsDto {
    pub challenges: Vec<ChallengesDto>,
}

#[derive(Clone)]
pub struct ChallengesDto {
    pub worker_index: u32,
    pub airgroup_id: u32,
    pub challenge: Vec<u64>,
}

pub struct AggParamsDto {
    pub agg_proofs: Vec<ProofDto>,
    pub last_proof: bool,
    pub final_proof: bool,
    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,
    pub verify_proofs: bool,
    pub save_proofs: bool,
    pub test_mode: bool,
    pub output_dir_path: String,
    pub minimal_memory: bool,
}

pub struct ProofDto {
    pub worker_idx: u32,
    pub airgroup_id: u64,
    pub values: Vec<u64>,
}

pub struct ExecuteTaskResponseDto {
    pub job_id: JobId,
    pub prover_id: ProverId,
    pub success: bool,
    pub error_message: Option<String>,
    pub result_data: ExecuteTaskResponseResultDataDto,
}

pub enum ExecuteTaskResponseResultDataDto {
    Challenges(Vec<ChallengesDto>),
    Proofs(Vec<ProofDto>),
    FinalProof(Vec<u64>),
}

pub struct HeartbeatAckDto {
    pub prover_id: ProverId,
}

pub struct ProverErrorDto {
    pub prover_id: ProverId,
    pub job_id: JobId,
    pub error_message: String,
}
