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
    contribution_params::InputSource, coordinator_message::Payload, execute_task_request,
    execute_task_response, job_status_response, jobs_list_response, launch_proof_response,
    system_status_response, workers_list_response, AggParams, Challenges,
    ComputeCapacity as GrpcComputeCapacity, ContributionParams, CoordinatorMessage,
    ExecuteTaskRequest, ExecuteTaskResponse, Heartbeat, HeartbeatAck, HintsMode, InputMode,
    JobCancelled, JobStatus, JobStatusResponse, JobsList, JobsListResponse, LaunchProofRequest,
    LaunchProofResponse, Metrics, Proof, ProofList, ProveParams, ProgramInfo, ProgramStatus,
    DeleteProgram, ProgramSetupAck, RegisterProgram, RegisterProgramRequest, RegisterProgramResponse,
    UpdateProgramRequest, UpdateProgramResponse,
    Shutdown, StatusInfoResponse, StreamData, StreamPayload, StreamType, SystemStatus,
    SystemStatusResponse, TaskType, WorkerError, WorkerInfo, WorkerReconnectRequest,
    WorkerRegisterRequest, WorkerRegisterResponse, WorkersList, WorkersListResponse,
};
use zisk_distributed_common::*;

use anyhow::Result;

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
            data_id: dto.data_id.into(),
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
            data_id: dto.data_id.into(),
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
        let (inputs_mode, inputs_uri) = match dto.inputs_mode {
            InputsModeDto::InputsNone => (InputMode::None, None),
            InputsModeDto::InputsPath(inputs_path) => (InputMode::Path, Some(inputs_path)),
            InputsModeDto::InputsData(inputs_uri) => (InputMode::Data, Some(inputs_uri)),
        };

        let (hints_mode, hints_uri) = match dto.hints_mode {
            HintsModeDto::HintsNone => (HintsMode::None, None),
            HintsModeDto::HintsPath(hints_path) => (HintsMode::Path, Some(hints_path)),
            HintsModeDto::HintsStream(hints_uri) => (HintsMode::Stream, Some(hints_uri)),
        };

        LaunchProofRequest {
            data_id: dto.data_id.into(),
            compute_capacity: dto.compute_capacity,
            minimal_compute_capacity: dto.minimal_compute_capacity,
            inputs_mode: inputs_mode.into(),
            inputs_uri,
            hints_mode: hints_mode.into(),
            hints_uri,
            simulated_node: dto.simulated_node,
        }
    }
}

use std::convert::TryFrom;
use std::sync::Arc;

impl TryFrom<LaunchProofRequest> for LaunchProofRequestDto {
    type Error = anyhow::Error;

    fn try_from(req: LaunchProofRequest) -> Result<Self> {
        Ok(LaunchProofRequestDto {
            data_id: req.data_id.into(),
            compute_capacity: req.compute_capacity,
            minimal_compute_capacity: req.minimal_compute_capacity,
            inputs_mode: match InputMode::try_from(req.inputs_mode).unwrap_or(InputMode::None) {
                InputMode::None => InputsModeDto::InputsNone,
                InputMode::Path => {
                    let inputs_uri = req.inputs_uri.ok_or_else(|| {
                        anyhow::anyhow!("Input mode is Uri but inputs_uri is missing")
                    })?;
                    InputsModeDto::InputsPath(inputs_uri)
                }
                InputMode::Data => {
                    let inputs_uri = req.inputs_uri.ok_or_else(|| {
                        anyhow::anyhow!("Input mode is Data but inputs_uri is missing")
                    })?;
                    InputsModeDto::InputsData(inputs_uri)
                }
            },
            hints_mode: match HintsMode::try_from(req.hints_mode).unwrap_or(HintsMode::None) {
                HintsMode::None => HintsModeDto::HintsNone,
                HintsMode::Path => {
                    let hints_uri = req.hints_uri.ok_or_else(|| {
                        anyhow::anyhow!("Hints mode is Uri but hints_uri is missing")
                    })?;
                    HintsModeDto::HintsPath(hints_uri)
                }
                HintsMode::Stream => {
                    let hints_uri = req.hints_uri.ok_or_else(|| {
                        anyhow::anyhow!("Hints mode is Stream but hints_uri is missing")
                    })?;
                    HintsModeDto::HintsStream(hints_uri)
                }
            },
            simulated_node: req.simulated_node,
        })
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
            CoordinatorMessageDto::StreamData(data) => {
                CoordinatorMessage { payload: Some(Payload::StreamData(data.into())) }
            }
            CoordinatorMessageDto::RegisterProgram(msg) => {
                CoordinatorMessage { payload: Some(Payload::RegisterProgram(msg.into())) }
            }
            CoordinatorMessageDto::DeleteProgram(msg) => {
                CoordinatorMessage { payload: Some(Payload::DeleteProgram(msg.into())) }
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
        let input_source = match dto.input_source {
            InputSourceDto::InputPath(inputs_path) => Some(InputSource::InputPath(inputs_path)),
            InputSourceDto::InputData(data) => Some(InputSource::InputData(data)),
            InputSourceDto::InputNull => None,
        };

        let (hints_path, hints_stream) = match dto.hints_source {
            HintsSourceDto::HintsPath(hints_path) => (Some(hints_path), false),
            HintsSourceDto::HintsStream(hints_path) => (Some(hints_path), true),
            HintsSourceDto::HintsNull => (None, false),
        };

        ContributionParams {
            data_id: dto.data_id.as_string(),
            input_source,
            hints_path,
            hints_stream,
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
            compressed: dto.compressed,
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
                let witness_info = challenges_list.witness_info.unwrap();
                let witness_info = WitnessInfoDto {
                    witness_time: witness_info.witness_time,
                    publics: witness_info.publics,
                    proof_values: witness_info.proof_values,
                    summary_info: witness_info.summary_info,
                };
                let exec_time = challenges_list.zisk_execution_time.unwrap();
                let zisk_executor_time = ZiskExecutorTimeDto {
                    task_received_time: exec_time.task_received_time,
                    total_duration: exec_time.total_duration,
                    execution_duration: exec_time.execution_duration,
                    count_and_plan_duration: exec_time.count_and_plan_duration,
                    count_and_plan_mo_duration: exec_time.count_and_plan_mo_duration,
                    asm_execution_duration: exec_time.asm_execution_duration.map(|asm_info| {
                        AsmExecutionInfoDto { time: asm_info.time, mhz: asm_info.mhz }
                    }),
                };

                Some(ExecuteTaskResponseResultDataDto::Challenges(ContributionsResultDataDto {
                    witness_info,
                    challenges,
                    zisk_executor_time,
                }))
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

impl From<StreamMessageKind> for StreamType {
    fn from(dto: StreamMessageKind) -> StreamType {
        match dto {
            StreamMessageKind::Start => StreamType::Start,
            StreamMessageKind::Data => StreamType::Data,
            StreamMessageKind::End => StreamType::End,
        }
    }
}

impl From<StreamType> for StreamMessageKind {
    fn from(stream_type: StreamType) -> StreamMessageKind {
        match stream_type {
            StreamType::Start => StreamMessageKind::Start,
            StreamType::Data => StreamMessageKind::Data,
            StreamType::End => StreamMessageKind::End,
        }
    }
}

impl From<StreamDataDto> for StreamData {
    fn from(dto: StreamDataDto) -> Self {
        StreamData {
            job_id: dto.job_id.as_string(),
            stream_type: StreamType::from(dto.stream_type) as i32,
            payload: dto.stream_payload.map(Into::into),
        }
    }
}

impl From<StreamData> for StreamDataDto {
    fn from(data: StreamData) -> Self {
        StreamDataDto {
            job_id: JobId::from(data.job_id),
            stream_type: StreamType::try_from(data.stream_type)
                .map(StreamMessageKind::from)
                .unwrap_or(StreamMessageKind::Data),
            stream_payload: data.payload.map(Into::into),
        }
    }
}

impl From<StreamPayloadDto> for StreamPayload {
    fn from(dto: StreamPayloadDto) -> Self {
        StreamPayload { sequence_number: dto.sequence_number, payload: dto.payload }
    }
}

impl From<StreamPayload> for StreamPayloadDto {
    fn from(payload: StreamPayload) -> Self {
        StreamPayloadDto { sequence_number: payload.sequence_number, payload: payload.payload }
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

// ── Program conversions ───────────────────────────────────────────────────────

fn program_status_to_i32(status: ProgramStatusDto) -> i32 {
    match status {
        ProgramStatusDto::Provisioning => ProgramStatus::Provisioning as i32,
        ProgramStatusDto::Ready => ProgramStatus::Ready as i32,
        ProgramStatusDto::Failed => ProgramStatus::Failed as i32,
    }
}

fn i32_to_program_status(value: i32) -> ProgramStatusDto {
    match ProgramStatus::try_from(value).unwrap_or(ProgramStatus::Provisioning) {
        ProgramStatus::Provisioning => ProgramStatusDto::Provisioning,
        ProgramStatus::Ready => ProgramStatusDto::Ready,
        ProgramStatus::Failed => ProgramStatusDto::Failed,
    }
}

impl From<RegisterProgramRequestDto> for RegisterProgramRequest {
    fn from(dto: RegisterProgramRequestDto) -> Self {
        RegisterProgramRequest {
            name: dto.name,
            description: dto.description,
            author: dto.author,
            zisk_elf: dto.zisk_elf,
            metadata: dto.metadata,
        }
    }
}

impl From<RegisterProgramRequest> for RegisterProgramRequestDto {
    fn from(req: RegisterProgramRequest) -> Self {
        RegisterProgramRequestDto {
            name: req.name,
            description: req.description,
            author: req.author,
            zisk_elf: req.zisk_elf,
            metadata: req.metadata,
        }
    }
}

impl From<RegisterProgramResponseDto> for RegisterProgramResponse {
    fn from(dto: RegisterProgramResponseDto) -> Self {
        RegisterProgramResponse {
            hash_id: dto.hash_id,
            program_id: dto.program_id,
            status: program_status_to_i32(dto.status),
        }
    }
}

impl From<RegisterProgramResponse> for RegisterProgramResponseDto {
    fn from(resp: RegisterProgramResponse) -> Self {
        RegisterProgramResponseDto {
            hash_id: resp.hash_id,
            program_id: resp.program_id,
            status: i32_to_program_status(resp.status),
        }
    }
}

impl From<ProgramInfoDto> for ProgramInfo {
    fn from(dto: ProgramInfoDto) -> Self {
        ProgramInfo {
            program_id: dto.program_id,
            hash_id: dto.hash_id,
            name: dto.name,
            description: dto.description,
            author: dto.author,
            status: program_status_to_i32(dto.status),
            metadata: dto.metadata,
            created_at: Some(prost_types::Timestamp {
                seconds: dto.created_at.timestamp(),
                nanos: dto.created_at.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

impl From<ProgramInfo> for ProgramInfoDto {
    fn from(info: ProgramInfo) -> Self {
        let created_at = info
            .created_at
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .unwrap_or_else(chrono::Utc::now)
            })
            .unwrap_or_else(chrono::Utc::now);
        ProgramInfoDto {
            program_id: info.program_id,
            hash_id: info.hash_id,
            name: info.name,
            description: info.description,
            author: info.author,
            status: i32_to_program_status(info.status),
            metadata: info.metadata,
            created_at,
        }
    }
}

impl From<UpdateProgramRequestDto> for UpdateProgramRequest {
    fn from(dto: UpdateProgramRequestDto) -> Self {
        UpdateProgramRequest {
            program_id: dto.program_id,
            name: dto.name,
            description: dto.description,
            author: dto.author,
            metadata: dto.metadata,
            zisk_elf: dto.zisk_elf,
        }
    }
}

impl From<UpdateProgramRequest> for UpdateProgramRequestDto {
    fn from(req: UpdateProgramRequest) -> Self {
        UpdateProgramRequestDto {
            program_id: req.program_id,
            name: req.name,
            description: req.description,
            author: req.author,
            metadata: req.metadata,
            zisk_elf: req.zisk_elf,
        }
    }
}

impl From<UpdateProgramResponseDto> for UpdateProgramResponse {
    fn from(dto: UpdateProgramResponseDto) -> Self {
        UpdateProgramResponse {
            program_id: dto.program_id,
            hash_id: dto.hash_id,
            status: program_status_to_i32(dto.status),
        }
    }
}

impl From<UpdateProgramResponse> for UpdateProgramResponseDto {
    fn from(resp: UpdateProgramResponse) -> Self {
        UpdateProgramResponseDto {
            program_id: resp.program_id,
            hash_id: resp.hash_id,
            status: i32_to_program_status(resp.status),
        }
    }
}

// Cluster stream: coordinator → worker
impl From<RegisterProgramMessageDto> for RegisterProgram {
    fn from(dto: RegisterProgramMessageDto) -> Self {
        RegisterProgram {
            name: dto.name,
            program_id: dto.program_id,
            hash_id: dto.hash_id,
            zisk_elf: (*dto.zisk_elf).clone(),
        }
    }
}

impl From<RegisterProgram> for RegisterProgramMessageDto {
    fn from(msg: RegisterProgram) -> Self {
        RegisterProgramMessageDto {
            name: msg.name,
            program_id: msg.program_id,
            hash_id: msg.hash_id,
            zisk_elf: Arc::new(msg.zisk_elf),
        }
    }
}

// Cluster stream: coordinator → worker (program delete)
impl From<DeleteProgramMessageDto> for DeleteProgram {
    fn from(dto: DeleteProgramMessageDto) -> Self {
        DeleteProgram { name: dto.name, program_id: dto.program_id, hash_id: dto.hash_id }
    }
}

impl From<DeleteProgram> for DeleteProgramMessageDto {
    fn from(msg: DeleteProgram) -> Self {
        DeleteProgramMessageDto { name: msg.name, program_id: msg.program_id, hash_id: msg.hash_id }
    }
}

// Cluster stream: worker → coordinator
impl From<ProgramSetupAckDto> for ProgramSetupAck {
    fn from(dto: ProgramSetupAckDto) -> Self {
        ProgramSetupAck {
            hash_id: dto.hash_id,
            success: dto.success,
            error: dto.error.unwrap_or_default(),
        }
    }
}

impl From<ProgramSetupAck> for ProgramSetupAckDto {
    fn from(ack: ProgramSetupAck) -> Self {
        ProgramSetupAckDto {
            hash_id: ack.hash_id,
            success: ack.success,
            error: if ack.error.is_empty() { None } else { Some(ack.error) },
        }
    }
}
