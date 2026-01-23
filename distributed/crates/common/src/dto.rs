//! Data Transfer Objects (DTOs) for Distributed Proving System
//!
//! This module defines the internal domain types used throughout the distributed proving system.
//! These DTOs serve as the canonical data structures for business logic, separate from external
//! representations like gRPC protobuf types or serialization formats.

use crate::{ComputeCapacity, DataId, JobId, JobPhase, JobState, WorkerId, WorkerState};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub data_id: DataId,
    pub state: JobState,
    pub phase: Option<JobPhase>,
    pub assigned_workers: Vec<WorkerId>,
    pub start_time: u64,
    pub duration_ms: u64,
}

pub struct WorkersListDto {
    pub workers: Vec<WorkerInfoDto>,
}

pub struct WorkerInfoDto {
    pub worker_id: WorkerId,
    pub state: WorkerState,
    pub compute_capacity: ComputeCapacity,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

pub struct SystemStatusDto {
    pub total_workers: u32,
    pub compute_capacity: ComputeCapacity,
    pub idle_workers: u32,
    pub busy_workers: u32,
    pub active_jobs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputsModeDto {
    // No inputs are provided
    InputsNone,
    /// Inputs are provided as a complete payload referenced by a URI.
    InputsPath(String),
    /// Inputs are provided directly as data.
    InputsData(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HintsModeDto {
    /// No hints are provided.
    HintsNone,
    /// Hints are provided as a complete payload referenced by a URI.
    HintsPath(String),
    /// Hints will be streamed from the given URI endpoint.
    HintsStream(String),
}

pub struct LaunchProofRequestDto {
    pub data_id: DataId,
    pub compute_capacity: u32,
    pub inputs_mode: InputsModeDto,
    pub hints_mode: HintsModeDto,
    pub simulated_node: Option<u32>,
}

pub struct LaunchProofResponseDto {
    pub job_id: JobId,
}

pub struct MetricsDto {
    pub active_connections: u32,
}

pub struct WorkerRegisterRequestDto {
    pub worker_id: WorkerId,
    pub compute_capacity: ComputeCapacity,
}

pub struct WorkerReconnectRequestDto {
    pub worker_id: WorkerId,
    pub compute_capacity: ComputeCapacity,
}

pub enum CoordinatorMessageDto {
    Heartbeat(HeartbeatDto),
    Shutdown(ShutdownDto),
    WorkerRegisterResponse(WorkerRegisterResponseDto),
    ExecuteTaskRequest(ExecuteTaskRequestDto),
    JobCancelled(JobCancelledDto),
    StreamData(StreamDataDto),
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
}

pub struct ContributionParamsDto {
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
    HintsStream(String),
    HintsNull,
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
    pub rma: bool,
    pub compressed: bool,
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

pub struct FinalProofDto {
    pub values: Vec<u64>,
    pub executed_steps: u64,
}

pub struct ExecuteTaskResponseDto {
    pub job_id: JobId,
    pub worker_id: WorkerId,
    pub success: bool,
    pub error_message: Option<String>,
    pub result_data: ExecuteTaskResponseResultDataDto,
}

pub enum ExecuteTaskResponseResultDataDto {
    Challenges(Vec<ChallengesDto>),
    Proofs(Vec<ProofDto>),
    FinalProof(FinalProofDto),
}

pub struct HeartbeatAckDto {
    pub worker_id: WorkerId,
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
    pub proof: Option<Vec<u64>>,
    pub executed_steps: Option<u64>,
    pub timestamp: String,
    pub error: Option<WebhookErrorDto>,
}

impl WebhookPayloadDto {
    /// Creates a successful webhook payload
    pub fn success(
        job_id: String,
        duration_ms: u64,
        proof: Option<Vec<u64>>,
        executed_steps: Option<u64>,
    ) -> Self {
        Self {
            job_id,
            success: true,
            duration_ms,
            proof,
            executed_steps,
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: None,
        }
    }

    /// Creates a failed webhook payload with error details
    pub fn failure(job_id: String, duration_ms: u64, error: WebhookErrorDto) -> Self {
        Self {
            job_id,
            success: false,
            duration_ms,
            proof: None,
            executed_steps: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: Some(error),
        }
    }
}
