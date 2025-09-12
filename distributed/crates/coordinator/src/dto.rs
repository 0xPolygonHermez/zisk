use chrono::{DateTime, Utc};
use distributed_common::ComputeCapacity;
use distributed_grpc_api::{
    job_status_response, jobs_list_response, provers_list_response, start_proof_response,
    JobStatus, JobStatusResponse, JobsListResponse, Metrics, ProversListResponse,
    StartProofResponse, StatusInfoResponse, SystemStatusResponse,
};

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

/// Alternative implementation using From trait for more idiomatic conversion
impl From<StatusInfoDto> for StatusInfoResponse {
    fn from(dto: StatusInfoDto) -> Self {
        StatusInfoResponse {
            service_name: dto.service_name,
            version: dto.version,
            uptime_seconds: dto.uptime_seconds,
            start_time: Some(prost_types::Timestamp {
                seconds: dto.start_time.timestamp(),
                nanos: dto.start_time.timestamp_subsec_nanos() as i32,
            }),
            metrics: Some(dto.metrics.into()),
        }
    }
}

pub struct JobsListDto {
    pub jobs: Vec<JobStatusDto>,
}

impl From<JobsListDto> for JobsListResponse {
    fn from(dto: JobsListDto) -> Self {
        let job_statuses: Vec<distributed_grpc_api::JobStatus> =
            dto.jobs.into_iter().map(|job| job.into()).collect();
        let jobs_list = distributed_grpc_api::JobsList { jobs: job_statuses };
        JobsListResponse { result: Some(jobs_list_response::Result::JobsList(jobs_list)) }
    }
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

impl From<JobStatusDto> for JobStatus {
    fn from(dto: JobStatusDto) -> Self {
        JobStatus {
            job_id: dto.job_id,
            block_id: dto.block_id,
            phase: dto.phase,
            status: dto.status,
            assigned_provers: dto.assigned_provers,
            start_time: dto.start_time,
            duration_ms: dto.duration_ms,
        }
    }
}

impl From<JobStatusDto> for JobStatusResponse {
    fn from(dto: JobStatusDto) -> Self {
        let job_status = JobStatus {
            job_id: dto.job_id,
            block_id: dto.block_id,
            phase: dto.phase,
            status: dto.status,
            assigned_provers: dto.assigned_provers,
            start_time: dto.start_time,
            duration_ms: dto.duration_ms,
        };
        JobStatusResponse { result: Some(job_status_response::Result::Job(job_status)) }
    }
}

pub struct ProversListDto {
    pub provers: Vec<ProverStatusDto>,
}

impl From<ProversListDto> for ProversListResponse {
    fn from(dto: ProversListDto) -> Self {
        let prover_statuses: Vec<distributed_grpc_api::ProverStatus> =
            dto.provers.into_iter().map(|prover| prover.into()).collect();
        let provers_list = distributed_grpc_api::ProversList { provers: prover_statuses };
        ProversListResponse {
            result: Some(provers_list_response::Result::ProversList(provers_list)),
        }
    }
}

pub struct ProverStatusDto {
    pub prover_id: String,
    pub state: String,
    pub current_job_id: String,
    pub allocated_capacity: ComputeCapacity,
    pub last_heartbeat: u64,
    pub jobs_completed: u32,
}

impl From<ProverStatusDto> for distributed_grpc_api::ProverStatus {
    fn from(dto: ProverStatusDto) -> Self {
        distributed_grpc_api::ProverStatus {
            prover_id: dto.prover_id,
            state: dto.state,
            current_job_id: dto.current_job_id,
            allocated_capacity: Some(dto.allocated_capacity.into()),
            last_heartbeat: dto.last_heartbeat,
            jobs_completed: dto.jobs_completed,
        }
    }
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

impl From<SystemStatusDto> for distributed_grpc_api::SystemStatusResponse {
    fn from(dto: SystemStatusDto) -> Self {
        let system_status = distributed_grpc_api::SystemStatus {
            total_provers: dto.total_provers,
            compute_capacity: dto.compute_capacity.compute_units,
            idle_provers: dto.idle_provers,
            busy_provers: dto.busy_provers,
            active_jobs: dto.active_jobs,
            pending_jobs: dto.pending_jobs,
            completed_jobs_last_minute: dto.completed_jobs_last_minute,
            job_completion_rate: dto.job_completion_rate,
            prover_utilization: dto.prover_utilization,
        };

        SystemStatusResponse {
            result: Some(distributed_grpc_api::system_status_response::Result::Status(
                system_status,
            )),
        }
    }
}

pub struct StartProofRequestDto {
    pub block_id: String,
    pub compute_units: u32,
    pub input_path: String,
}

impl From<StartProofRequestDto> for distributed_grpc_api::StartProofRequest {
    fn from(dto: StartProofRequestDto) -> Self {
        distributed_grpc_api::StartProofRequest {
            block_id: dto.block_id,
            compute_units: dto.compute_units,
            input_path: dto.input_path,
        }
    }
}

impl From<distributed_grpc_api::StartProofRequest> for StartProofRequestDto {
    fn from(request: distributed_grpc_api::StartProofRequest) -> Self {
        StartProofRequestDto {
            block_id: request.block_id,
            compute_units: request.compute_units,
            input_path: request.input_path,
        }
    }
}

pub struct StartProofResponseDto {
    pub job_id: String,
}

impl From<StartProofResponseDto> for StartProofResponse {
    fn from(dto: StartProofResponseDto) -> Self {
        StartProofResponse { result: Some(start_proof_response::Result::JobId(dto.job_id)) }
    }
}

pub struct MetricsDto {
    pub active_connections: u32,
}

impl From<MetricsDto> for Metrics {
    fn from(dto: MetricsDto) -> Self {
        distributed_grpc_api::Metrics { active_connections: dto.active_connections }
    }
}

pub struct ProverRegisterRequestDto {
    pub prover_id: String,
    pub compute_capacity: ComputeCapacity,
}

impl From<distributed_grpc_api::ProverRegisterRequest> for ProverRegisterRequestDto {
    fn from(request: distributed_grpc_api::ProverRegisterRequest) -> Self {
        ProverRegisterRequestDto {
            prover_id: request.prover_id,
            compute_capacity: ComputeCapacity {
                compute_units: request.compute_capacity.unwrap().compute_units,
            },
        }
    }
}

pub struct ProverReconnectRequestDto {
    pub prover_id: String,
    pub compute_capacity: ComputeCapacity,
}

impl From<distributed_grpc_api::ProverReconnectRequest> for ProverReconnectRequestDto {
    fn from(request: distributed_grpc_api::ProverReconnectRequest) -> Self {
        ProverReconnectRequestDto {
            prover_id: request.prover_id,
            compute_capacity: ComputeCapacity {
                compute_units: request.compute_capacity.unwrap().compute_units,
            },
        }
    }
}
