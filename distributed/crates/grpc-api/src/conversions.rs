//! Type Conversions for Distributed Proving System gRPC API
//!
//! This module provides bidirectional type conversions between the domain types used in the
//! distributed proving system (`distributed-common`) and the generated gRPC protobuf types.
//!
//! ## Purpose
//!
//! The gRPC protobuf compiler generates Rust types that don't always match our internal domain
//! model. All conversions implement the `From` and/or `Into` traits for idiomatic Rust usage.

use crate::{
    coordinator_message::Payload, execute_task_request, execute_task_response, job_status_response,
    jobs_list_response, launch_proof_response, system_status_response, workers_list_response,
    AggParams, Challenges, ComputeCapacity as GrpcComputeCapacity, ContributionParams,
    CoordinatorMessage, ExecuteTaskRequest, ExecuteTaskResponse, Heartbeat, HeartbeatAck,
    JobCancelled, JobStatus, JobStatusResponse, JobsList, JobsListResponse, LaunchProofRequest,
    LaunchProofResponse, Metrics, Proof, ProofList, ProveParams, Shutdown, StatusInfoResponse,
    SystemStatus, SystemStatusResponse, TaskType, WorkerError, WorkerInfo, WorkerReconnectRequest,
    WorkerRegisterRequest, WorkerRegisterResponse, WorkersList, WorkersListResponse,
};
use zisk_distributed_common::*;

impl From<ComputeCapacity> for GrpcComputeCapacity {
    fn from(capacity: ComputeCapacity) -> Self {
        GrpcComputeCapacity { compute_units: capacity.compute_units }
    }
}

impl From<GrpcComputeCapacity> for ComputeCapacity {
    fn from(grpc_capacity: GrpcComputeCapacity) -> Self {
        ComputeCapacity::from(grpc_capacity.compute_units)
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
            job_id: dto.job_id.into(),
            block_id: dto.block_id.into(),
            phase: dto.phase.map_or("None".to_string(), |p| p.to_string()),
            state: dto.state.to_string(),
            assigned_workers: dto.assigned_workers.into_iter().map(|id| id.into()).collect(),
            start_time: dto.start_time,
            duration_ms: dto.duration_ms,
        }
    }
}

impl From<JobStatusDto> for JobStatusResponse {
    fn from(dto: JobStatusDto) -> Self {
        let job_status = JobStatus {
            job_id: dto.job_id.into(),
            block_id: dto.block_id.into(),
            phase: dto.phase.map_or("None".to_string(), |p| p.to_string()),
            state: dto.state.to_string(),
            assigned_workers: dto.assigned_workers.into_iter().map(|id| id.into()).collect(),
            start_time: dto.start_time,
            duration_ms: dto.duration_ms,
        };
        JobStatusResponse { result: Some(job_status_response::Result::Job(job_status)) }
    }
}

impl From<WorkersListDto> for WorkersListResponse {
    fn from(dto: WorkersListDto) -> Self {
        let workers_info: Vec<WorkerInfo> =
            dto.workers.into_iter().map(|worker| worker.into()).collect();
        let workers_list = WorkersList { workers: workers_info };
        WorkersListResponse {
            result: Some(workers_list_response::Result::WorkersList(workers_list)),
        }
    }
}

impl From<WorkerInfoDto> for WorkerInfo {
    fn from(dto: WorkerInfoDto) -> Self {
        WorkerInfo {
            worker_id: dto.worker_id.into(),
            state: dto.state.to_string(),
            compute_capacity: Some(dto.compute_capacity.into()),
            last_heartbeat: Some(prost_types::Timestamp {
                seconds: dto.last_heartbeat.timestamp(),
                nanos: dto.last_heartbeat.timestamp_subsec_nanos() as i32,
            }),
            connected_at: Some(prost_types::Timestamp {
                seconds: dto.connected_at.timestamp(),
                nanos: dto.connected_at.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

impl From<SystemStatusDto> for SystemStatusResponse {
    fn from(dto: SystemStatusDto) -> Self {
        let system_status = SystemStatus {
            total_workers: dto.total_workers,
            compute_capacity: dto.compute_capacity.compute_units,
            idle_workers: dto.idle_workers,
            busy_workers: dto.busy_workers,
            active_jobs: dto.active_jobs,
        };

        SystemStatusResponse { result: Some(system_status_response::Result::Status(system_status)) }
    }
}

impl From<LaunchProofRequestDto> for LaunchProofRequest {
    fn from(dto: LaunchProofRequestDto) -> Self {
        LaunchProofRequest {
            block_id: dto.block_id.into(),
            compute_capacity: dto.compute_capacity,
            input_path: dto.input_path,
            simulated_node: dto.simulated_node,
        }
    }
}

impl From<LaunchProofRequest> for LaunchProofRequestDto {
    fn from(req: LaunchProofRequest) -> Self {
        LaunchProofRequestDto {
            block_id: req.block_id.into(),
            compute_capacity: req.compute_capacity,
            input_path: req.input_path,
            simulated_node: req.simulated_node,
        }
    }
}

impl From<LaunchProofResponseDto> for LaunchProofResponse {
    fn from(dto: LaunchProofResponseDto) -> Self {
        LaunchProofResponse {
            result: Some(launch_proof_response::Result::JobId(dto.job_id.into())),
        }
    }
}

impl From<MetricsDto> for Metrics {
    fn from(dto: MetricsDto) -> Self {
        Metrics { active_connections: dto.active_connections }
    }
}

impl From<WorkerRegisterRequest> for WorkerRegisterRequestDto {
    fn from(req: WorkerRegisterRequest) -> Self {
        WorkerRegisterRequestDto {
            worker_id: req.worker_id.into(),
            compute_capacity: ComputeCapacity::from(req.compute_capacity.unwrap()),
        }
    }
}

impl From<WorkerReconnectRequest> for WorkerReconnectRequestDto {
    fn from(req: WorkerReconnectRequest) -> Self {
        WorkerReconnectRequestDto {
            worker_id: req.worker_id.into(),
            compute_capacity: ComputeCapacity::from(req.compute_capacity.unwrap()),
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
            CoordinatorMessageDto::WorkerRegisterResponse(resp) => {
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

impl From<WorkerRegisterResponseDto> for WorkerRegisterResponse {
    fn from(dto: WorkerRegisterResponseDto) -> Self {
        WorkerRegisterResponse {
            worker_id: dto.worker_id.as_string(),
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
            worker_id: dto.worker_id.into(),
            job_id: dto.job_id.into(),
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
            total_workers: dto.total_workers,
            worker_allocation: dto.worker_allocation,
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
                Some(ExecuteTaskResponseResultDataDto::FinalProof(FinalProofDto {
                    values: final_proof.values,
                    executed_steps: final_proof.executed_steps,
                }))
            }
            None => None,
        };

        ExecuteTaskResponseDto {
            job_id: JobId::from(response.job_id),
            worker_id: WorkerId::from(response.worker_id),
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
        HeartbeatAckDto { worker_id: WorkerId::from(message.worker_id) }
    }
}

impl From<WorkerError> for WorkerErrorDto {
    fn from(error: WorkerError) -> Self {
        WorkerErrorDto {
            worker_id: WorkerId::from(error.worker_id),
            job_id: JobId::from(error.job_id),
            error_message: error.error_message,
        }
    }
}
