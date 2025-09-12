use crate::{BlockId, ComputeCapacity, JobId, ProverId};
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
    pub job_id: String,
    pub block_id: String,
    pub phase: String,
    pub status: String,
    pub assigned_provers: Vec<String>,
    pub start_time: u64,
    pub duration_ms: u64,
}

pub struct ProversListDto {
    pub provers: Vec<ProverStatusDto>,
}

pub struct ProverStatusDto {
    pub prover_id: String,
    pub state: String,
    pub current_job_id: String,
    pub allocated_capacity: ComputeCapacity,
    pub last_heartbeat: u64,
    pub jobs_completed: u32,
}

pub struct SystemStatusDto {
    pub total_provers: u32,
    pub compute_capacity: ComputeCapacity,
    pub idle_provers: u32,
    pub busy_provers: u32,
    pub active_jobs: u32,
    pub pending_jobs: u32,
    pub completed_jobs_last_minute: u32,
    pub job_completion_rate: f64,
    pub prover_utilization: f64,
}

pub struct StartProofRequestDto {
    pub block_id: String,
    pub compute_units: u32,
    pub input_path: String,
}

pub struct StartProofResponseDto {
    pub job_id: String,
}

pub struct MetricsDto {
    pub active_connections: u32,
}

pub struct ProverRegisterRequestDto {
    pub prover_id: String,
    pub compute_capacity: ComputeCapacity,
}

pub struct ProverReconnectRequestDto {
    pub prover_id: String,
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
    pub prover_id: String,
    pub job_id: String,
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
