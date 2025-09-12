use chrono::{DateTime, Utc};
use distributed_common::{BlockId, ComputeCapacity, JobId, ProverId};
use distributed_grpc_api::{
    coordinator_message::Payload, job_status_response, jobs_list_response, provers_list_response,
    start_proof_response, CoordinatorMessage, ExecuteTaskResponse, JobStatus, JobStatusResponse,
    JobsListResponse, Metrics, ProverError, ProversListResponse, StartProofResponse,
    StatusInfoResponse, SystemStatusResponse, TaskType,
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

////////////////////////////////////

pub enum CoordinatorMessageDto {
    Heartbeat(HeartbeatDto),
    Shutdown(ShutdownDto),
    ProverRegisterResponse(ProverRegisterResponseDto),
    ExecuteTaskRequest(ExecuteTaskRequestDto),
    JobCancelled(JobCancelledDto),
}

impl From<CoordinatorMessageDto> for CoordinatorMessage {
    fn from(dto: CoordinatorMessageDto) -> Self {
        match dto {
            CoordinatorMessageDto::Heartbeat(hb) => {
                CoordinatorMessage { payload: Some(Payload::Heartbeat(hb.into())) }
            }
            CoordinatorMessageDto::Shutdown(shutdown) => {
                CoordinatorMessage { payload: Some(Payload::Shutdown(shutdown.into())) }
            }
            CoordinatorMessageDto::ProverRegisterResponse(resp) => {
                CoordinatorMessage { payload: Some(Payload::RegisterResponse(resp.into())) }
            }
            CoordinatorMessageDto::ExecuteTaskRequest(req) => {
                CoordinatorMessage { payload: Some(Payload::ExecuteTask(req.into())) }
            }
            CoordinatorMessageDto::JobCancelled(cancel) => {
                CoordinatorMessage { payload: Some(Payload::JobCancelled(cancel.into())) }
            }
        }
    }
}

pub struct HeartbeatDto {
    timestamp: DateTime<Utc>,
}

impl From<HeartbeatDto> for distributed_grpc_api::Heartbeat {
    fn from(dto: HeartbeatDto) -> Self {
        distributed_grpc_api::Heartbeat {
            timestamp: Some(prost_types::Timestamp {
                seconds: dto.timestamp.timestamp(),
                nanos: dto.timestamp.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

pub struct ShutdownDto {
    reason: String,
    grace_period_seconds: u32,
}

impl From<ShutdownDto> for distributed_grpc_api::Shutdown {
    fn from(dto: ShutdownDto) -> Self {
        distributed_grpc_api::Shutdown {
            reason: dto.reason,
            grace_period_seconds: dto.grace_period_seconds,
        }
    }
}

pub struct ProverRegisterResponseDto {
    prover_id: ProverId,
    accepted: bool,
    message: String,
    registered_at: DateTime<Utc>,
}

impl From<ProverRegisterResponseDto> for distributed_grpc_api::ProverRegisterResponse {
    fn from(dto: ProverRegisterResponseDto) -> Self {
        distributed_grpc_api::ProverRegisterResponse {
            prover_id: dto.prover_id.as_string(),
            accepted: dto.accepted,
            message: dto.message,
            registered_at: Some(prost_types::Timestamp {
                seconds: dto.registered_at.timestamp(),
                nanos: dto.registered_at.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

pub struct JobCancelledDto {
    job_id: JobId,
    reason: String,
}

impl From<JobCancelledDto> for distributed_grpc_api::JobCancelled {
    fn from(dto: JobCancelledDto) -> Self {
        distributed_grpc_api::JobCancelled { job_id: dto.job_id.as_string(), reason: dto.reason }
    }
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

impl From<ExecuteTaskRequestDto> for distributed_grpc_api::ExecuteTaskRequest {
    fn from(dto: ExecuteTaskRequestDto) -> Self {
        let (params, task_type) = match dto.params {
            ExecuteTaskRequestTypeDto::ContributionParams(cp) => (
                distributed_grpc_api::execute_task_request::Params::ContributionParams(cp.into()),
                TaskType::PartialContribution,
            ),
            ExecuteTaskRequestTypeDto::ProveParams(pp) => (
                distributed_grpc_api::execute_task_request::Params::ProveParams(pp.into()),
                TaskType::Prove,
            ),
            ExecuteTaskRequestTypeDto::AggParams(ap) => (
                distributed_grpc_api::execute_task_request::Params::AggParams(ap.into()),
                TaskType::Aggregate,
            ),
        };

        distributed_grpc_api::ExecuteTaskRequest {
            prover_id: dto.prover_id,
            job_id: dto.job_id,
            task_type: task_type as i32,
            params: Some(params),
        }
    }
}

pub struct ContributionParamsDto {
    pub block_id: BlockId,
    pub input_path: String,
    pub rank_id: u32,
    pub total_provers: u32,
    pub prover_allocation: Vec<u32>,
    pub job_compute_units: ComputeCapacity,
}

impl From<ContributionParamsDto> for distributed_grpc_api::ContributionParams {
    fn from(dto: ContributionParamsDto) -> Self {
        distributed_grpc_api::ContributionParams {
            block_id: dto.block_id.as_string(),
            input_path: dto.input_path,
            rank_id: dto.rank_id,
            total_provers: dto.total_provers,
            prover_allocation: dto.prover_allocation,
            job_compute_units: dto.job_compute_units.compute_units,
        }
    }
}

pub struct ProveParamsDto {
    pub challenges: Vec<ChallengesDto>,
}

impl From<ProveParamsDto> for distributed_grpc_api::ProveParams {
    fn from(dto: ProveParamsDto) -> Self {
        let challenges: Vec<distributed_grpc_api::Challenges> =
            dto.challenges.into_iter().map(|c| c.into()).collect();

        distributed_grpc_api::ProveParams { challenges }
    }
}

#[derive(Clone)]
pub struct ChallengesDto {
    pub worker_index: u32,
    pub airgroup_id: u32,
    pub challenge: Vec<u64>,
}

impl From<ChallengesDto> for distributed_grpc_api::Challenges {
    fn from(dto: ChallengesDto) -> Self {
        distributed_grpc_api::Challenges {
            worker_index: dto.worker_index,
            airgroup_id: dto.airgroup_id,
            challenge: dto.challenge,
        }
    }
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

impl From<AggParamsDto> for distributed_grpc_api::AggParams {
    fn from(dto: AggParamsDto) -> Self {
        let agg_proofs: Vec<distributed_grpc_api::Proof> =
            dto.agg_proofs.into_iter().map(|proof| proof.into()).collect();

        distributed_grpc_api::AggParams {
            agg_proofs: Some(distributed_grpc_api::ProofList { proofs: agg_proofs }),
            last_proof: dto.last_proof,
            final_proof: dto.final_proof,
            verify_constraints: dto.verify_constraints,
            aggregation: dto.aggregation,
            final_snark: dto.final_snark,
            verify_proofs: dto.verify_proofs,
            save_proofs: dto.save_proofs,
            test_mode: dto.test_mode,
            output_dir_path: dto.output_dir_path,
            minimal_memory: dto.minimal_memory,
        }
    }
}

pub struct ProofDto {
    pub worker_idx: u32,
    pub airgroup_id: u64,
    pub values: Vec<u64>,
}

impl From<ProofDto> for distributed_grpc_api::Proof {
    fn from(dto: ProofDto) -> Self {
        distributed_grpc_api::Proof {
            worker_idx: dto.worker_idx,
            airgroup_id: dto.airgroup_id,
            values: dto.values,
        }
    }
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

impl From<ExecuteTaskResponse> for ExecuteTaskResponseDto {
    fn from(response: ExecuteTaskResponse) -> Self {
        let result_data = match response.result_data {
            Some(distributed_grpc_api::execute_task_response::ResultData::Challenges(
                challenges_list,
            )) => {
                let challenges: Vec<ChallengesDto> = challenges_list
                    .challenges
                    .into_iter()
                    .map(|c| ChallengesDto {
                        worker_index: c.worker_index,
                        airgroup_id: c.airgroup_id,
                        challenge: c.challenge,
                    })
                    .collect();
                Some(ExecuteTaskResponseResultDataDto::Challenges(challenges))
            }
            Some(distributed_grpc_api::execute_task_response::ResultData::Proofs(proof_list)) => {
                let proofs: Vec<ProofDto> = proof_list
                    .proofs
                    .into_iter()
                    .map(|p| ProofDto {
                        worker_idx: p.worker_idx,
                        airgroup_id: p.airgroup_id,
                        values: p.values,
                    })
                    .collect();
                Some(ExecuteTaskResponseResultDataDto::Proofs(proofs))
            }
            Some(distributed_grpc_api::execute_task_response::ResultData::FinalProof(
                final_proof,
            )) => Some(ExecuteTaskResponseResultDataDto::FinalProof(final_proof.values)),
            None => None,
        };

        ExecuteTaskResponseDto {
            job_id: JobId::from(response.job_id),
            prover_id: ProverId::from(response.prover_id),
            success: response.success,
            error_message: if response.error_message.is_empty() {
                None
            } else {
                Some(response.error_message)
            },
            result_data: result_data.unwrap(),
        }
    }
}

pub struct HeartbeatAckDto {
    pub prover_id: ProverId,
}

impl From<distributed_grpc_api::HeartbeatAck> for HeartbeatAckDto {
    fn from(message: distributed_grpc_api::HeartbeatAck) -> Self {
        HeartbeatAckDto { prover_id: ProverId::from(message.prover_id) }
    }
}

pub struct ProverErrorDto {
    pub prover_id: ProverId,
    pub job_id: JobId,
    pub error_message: String,
}

impl From<ProverError> for ProverErrorDto {
    fn from(error: ProverError) -> Self {
        ProverErrorDto {
            prover_id: ProverId::from(error.prover_id),
            job_id: JobId::from(error.job_id),
            error_message: error.error_message,
        }
    }
}
