use crate::{
    coordinator_message::Payload, execute_task_request, execute_task_response, job_status_response,
    jobs_list_response, provers_list_response, start_proof_response, system_status_response,
    AggParams, Challenges, ComputeCapacity as GrpcComputeCapacity, ContributionParams,
    CoordinatorMessage, ExecuteTaskRequest, ExecuteTaskResponse, Heartbeat, HeartbeatAck,
    JobCancelled, JobStatus, JobStatusResponse, JobsList, JobsListResponse, Metrics, Proof,
    ProofList, ProveParams, ProverError, ProverReconnectRequest, ProverRegisterRequest,
    ProverRegisterResponse, ProverStatus, ProversList, ProversListResponse, Shutdown,
    StartProofRequest, StartProofResponse, StatusInfoResponse, SystemStatus, SystemStatusResponse,
    TaskType,
};
use distributed_common::{
    AggParamsDto, AggProofData, ChallengesDto, ComputeCapacity, ContributionParamsDto,
    CoordinatorMessageDto, ExecuteTaskRequestDto, ExecuteTaskRequestTypeDto,
    ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, HeartbeatAckDto, HeartbeatDto,
    JobCancelledDto, JobId, JobStatusDto, JobsListDto, MetricsDto, ProofDto, ProveParamsDto,
    ProverErrorDto, ProverId, ProverReconnectRequestDto, ProverRegisterRequestDto,
    ProverRegisterResponseDto, ProverStatusDto, ProversListDto, ShutdownDto, StartProofRequestDto,
    StartProofResponseDto, StatusInfoDto, SystemStatusDto,
};

/// Conversions between coordinator-common types and gRPC types
/// This module handles the translation layer between our domain types
/// and the generated gRPC protobuf types.
impl From<ComputeCapacity> for GrpcComputeCapacity {
    fn from(capacity: ComputeCapacity) -> Self {
        GrpcComputeCapacity { compute_units: capacity.compute_units }
    }
}

impl From<GrpcComputeCapacity> for ComputeCapacity {
    fn from(grpc_capacity: GrpcComputeCapacity) -> Self {
        ComputeCapacity { compute_units: grpc_capacity.compute_units }
    }
}

impl From<AggProofData> for Proof {
    fn from(row_data: AggProofData) -> Self {
        Proof {
            airgroup_id: row_data.airgroup_id,
            values: row_data.values,
            worker_idx: row_data.worker_idx,
        }
    }
}

impl From<Proof> for AggProofData {
    fn from(grpc_row_data: Proof) -> Self {
        AggProofData {
            airgroup_id: grpc_row_data.airgroup_id,
            values: grpc_row_data.values,
            worker_idx: grpc_row_data.worker_idx,
        }
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

impl From<JobsListDto> for JobsListResponse {
    fn from(dto: JobsListDto) -> Self {
        let job_statuses: Vec<JobStatus> = dto.jobs.into_iter().map(|job| job.into()).collect();
        let jobs_list = JobsList { jobs: job_statuses };
        JobsListResponse { result: Some(jobs_list_response::Result::JobsList(jobs_list)) }
    }
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

impl From<ProversListDto> for ProversListResponse {
    fn from(dto: ProversListDto) -> Self {
        let prover_statuses: Vec<ProverStatus> =
            dto.provers.into_iter().map(|prover| prover.into()).collect();
        let provers_list = ProversList { provers: prover_statuses };
        ProversListResponse {
            result: Some(provers_list_response::Result::ProversList(provers_list)),
        }
    }
}

impl From<ProverStatusDto> for ProverStatus {
    fn from(dto: ProverStatusDto) -> Self {
        ProverStatus {
            prover_id: dto.prover_id,
            state: dto.state,
            current_job_id: dto.current_job_id,
            allocated_capacity: Some(dto.allocated_capacity.into()),
            last_heartbeat: dto.last_heartbeat,
            jobs_completed: dto.jobs_completed,
        }
    }
}

impl From<SystemStatusDto> for SystemStatusResponse {
    fn from(dto: SystemStatusDto) -> Self {
        let system_status = SystemStatus {
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

        SystemStatusResponse { result: Some(system_status_response::Result::Status(system_status)) }
    }
}

impl From<StartProofRequestDto> for StartProofRequest {
    fn from(dto: StartProofRequestDto) -> Self {
        StartProofRequest {
            block_id: dto.block_id,
            compute_units: dto.compute_units,
            input_path: dto.input_path,
        }
    }
}

impl From<StartProofRequest> for StartProofRequestDto {
    fn from(request: StartProofRequest) -> Self {
        StartProofRequestDto {
            block_id: request.block_id,
            compute_units: request.compute_units,
            input_path: request.input_path,
        }
    }
}

impl From<StartProofResponseDto> for StartProofResponse {
    fn from(dto: StartProofResponseDto) -> Self {
        StartProofResponse { result: Some(start_proof_response::Result::JobId(dto.job_id)) }
    }
}

impl From<MetricsDto> for Metrics {
    fn from(dto: MetricsDto) -> Self {
        Metrics { active_connections: dto.active_connections }
    }
}

impl From<ProverRegisterRequest> for ProverRegisterRequestDto {
    fn from(request: ProverRegisterRequest) -> Self {
        ProverRegisterRequestDto {
            prover_id: request.prover_id,
            compute_capacity: ComputeCapacity {
                compute_units: request.compute_capacity.unwrap().compute_units,
            },
        }
    }
}

impl From<ProverReconnectRequest> for ProverReconnectRequestDto {
    fn from(request: ProverReconnectRequest) -> Self {
        ProverReconnectRequestDto {
            prover_id: request.prover_id,
            compute_capacity: ComputeCapacity {
                compute_units: request.compute_capacity.unwrap().compute_units,
            },
        }
    }
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

impl From<HeartbeatDto> for Heartbeat {
    fn from(dto: HeartbeatDto) -> Self {
        Heartbeat {
            timestamp: Some(prost_types::Timestamp {
                seconds: dto.timestamp.timestamp(),
                nanos: dto.timestamp.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

impl From<ShutdownDto> for Shutdown {
    fn from(dto: ShutdownDto) -> Self {
        Shutdown { reason: dto.reason, grace_period_seconds: dto.grace_period_seconds }
    }
}

impl From<ProverRegisterResponseDto> for ProverRegisterResponse {
    fn from(dto: ProverRegisterResponseDto) -> Self {
        ProverRegisterResponse {
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

impl From<JobCancelledDto> for JobCancelled {
    fn from(dto: JobCancelledDto) -> Self {
        JobCancelled { job_id: dto.job_id.as_string(), reason: dto.reason }
    }
}

impl From<ExecuteTaskRequestDto> for ExecuteTaskRequest {
    fn from(dto: ExecuteTaskRequestDto) -> Self {
        let (params, task_type) = match dto.params {
            ExecuteTaskRequestTypeDto::ContributionParams(cp) => (
                execute_task_request::Params::ContributionParams(cp.into()),
                TaskType::PartialContribution,
            ),
            ExecuteTaskRequestTypeDto::ProveParams(pp) => {
                (execute_task_request::Params::ProveParams(pp.into()), TaskType::Prove)
            }
            ExecuteTaskRequestTypeDto::AggParams(ap) => {
                (execute_task_request::Params::AggParams(ap.into()), TaskType::Aggregate)
            }
        };

        ExecuteTaskRequest {
            prover_id: dto.prover_id,
            job_id: dto.job_id,
            task_type: task_type as i32,
            params: Some(params),
        }
    }
}

impl From<ContributionParamsDto> for ContributionParams {
    fn from(dto: ContributionParamsDto) -> Self {
        ContributionParams {
            block_id: dto.block_id.as_string(),
            input_path: dto.input_path,
            rank_id: dto.rank_id,
            total_provers: dto.total_provers,
            prover_allocation: dto.prover_allocation,
            job_compute_units: dto.job_compute_units.compute_units,
        }
    }
}

impl From<ProveParamsDto> for ProveParams {
    fn from(dto: ProveParamsDto) -> Self {
        let challenges: Vec<Challenges> = dto.challenges.into_iter().map(|c| c.into()).collect();

        ProveParams { challenges }
    }
}

impl From<ChallengesDto> for Challenges {
    fn from(dto: ChallengesDto) -> Self {
        Challenges {
            worker_index: dto.worker_index,
            airgroup_id: dto.airgroup_id,
            challenge: dto.challenge,
        }
    }
}

impl From<AggParamsDto> for AggParams {
    fn from(dto: AggParamsDto) -> Self {
        let agg_proofs: Vec<Proof> = dto.agg_proofs.into_iter().map(|proof| proof.into()).collect();

        AggParams {
            agg_proofs: Some(ProofList { proofs: agg_proofs }),
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

impl From<ProofDto> for Proof {
    fn from(dto: ProofDto) -> Self {
        Proof { worker_idx: dto.worker_idx, airgroup_id: dto.airgroup_id, values: dto.values }
    }
}

impl From<ExecuteTaskResponse> for ExecuteTaskResponseDto {
    fn from(response: ExecuteTaskResponse) -> Self {
        let result_data = match response.result_data {
            Some(execute_task_response::ResultData::Challenges(challenges_list)) => {
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
            Some(execute_task_response::ResultData::Proofs(proof_list)) => {
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
            Some(execute_task_response::ResultData::FinalProof(final_proof)) => {
                Some(ExecuteTaskResponseResultDataDto::FinalProof(final_proof.values))
            }
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

impl From<HeartbeatAck> for HeartbeatAckDto {
    fn from(message: HeartbeatAck) -> Self {
        HeartbeatAckDto { prover_id: ProverId::from(message.prover_id) }
    }
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
