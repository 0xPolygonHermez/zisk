//! Core Domain Types for Distributed Proving System
//!
//! This module defines the fundamental domain types and business entities used throughout
//! the distributed proving system. These types form the core vocabulary of the system,
//! providing type safety, semantic clarity, and domain-driven design principles.

use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use proofman::{ContributionsInfo, ProvePhaseInputs, WitnessInfo};
use proofman_common::ProofOptions;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt::{self, Debug, Display},
    ops::Range,
};
use tracing::error;
use zisk_common::{Proof, StatsCostPerType, ZiskExecutorTime};

use crate::{HintsModeDto, HintsSourceDto, InputSourceDto, InputsModeDto, ProofKind};

/// Job ID wrapper for type safety
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize,
)]
pub struct JobId(String);

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl JobId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for JobId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<JobId> for String {
    fn from(job_id: JobId) -> Self {
        job_id.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 8 {
            write!(f, "JobId({:.8}…)", self.0)
        } else {
            write!(f, "JobId({})", self.0)
        }
    }
}

/// Data ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DataId(String);

impl Default for DataId {
    fn default() -> Self {
        Self::new()
    }
}

impl DataId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for DataId {
    fn from(data_id: String) -> Self {
        Self(data_id)
    }
}

impl From<DataId> for String {
    fn from(data_id: DataId) -> Self {
        data_id.0
    }
}

impl std::fmt::Display for DataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 8 {
            write!(f, "DataId({:.8}…)", self.0)
        } else {
            write!(f, "DataId({})", self.0)
        }
    }
}

/// Worker ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkerId(String);

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for WorkerId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<WorkerId> for String {
    fn from(worker_id: WorkerId) -> Self {
        worker_id.0
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 8 {
            write!(f, "WorkerId({:.8}…)", self.0)
        } else {
            write!(f, "WorkerId({})", self.0)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState {
    Disconnected,
    Connecting,
    /// Connected but no setup done yet. Not eligible for job assignment.
    Idle,
    /// Running setup (guest program load). Not eligible for job assignment.
    SettingUp,
    /// Setup complete. Eligible for job assignment.
    Ready,
    Computing((JobId, JobPhase)),
    Error,
}

impl Display for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = match self {
            WorkerState::Disconnected => "Disconnected",
            WorkerState::Connecting => "Connecting",
            WorkerState::Idle => "Idle",
            WorkerState::SettingUp => "SettingUp",
            WorkerState::Ready => "Ready",
            WorkerState::Computing(phase) => return write!(f, "Computing({})", phase.1),
            WorkerState::Error => "Error",
        };
        write!(f, "{}", state_str)
    }
}

/// Compute capacity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComputeCapacity {
    pub compute_units: u32,
}

impl From<u32> for ComputeCapacity {
    fn from(units: u32) -> Self {
        Self { compute_units: units }
    }
}

impl std::fmt::Display for ComputeCapacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}CU", self.compute_units)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobExecutionMode {
    Standard,        // the normal mode
    Simulating(u32), // simulation mode where we simulate N workers but only use one worker
}

impl JobExecutionMode {
    pub fn is_simulating(&self) -> bool {
        matches!(self, JobExecutionMode::Simulating(_))
    }
}

/// Policy applied when a worker fails or a phase times out.
///
/// Determines how the coordinator reacts to failures during job execution.
/// The policy is configured at the coordinator level and applies to all jobs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum FailurePolicy {
    /// Abort the entire job immediately. All assigned workers are cancelled
    /// and the job is marked as failed.
    #[default]
    AbortJob,
    // /// Retry failed workers up to `max_retries` times before aborting.
    // /// If all retries are exhausted, the job is aborted.
    // RetryWorkers { max_retries: u32 },
}

impl Display for FailurePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FailurePolicy::AbortJob => write!(f, "AbortJob"),
            // FailurePolicy::RetryWorkers { max_retries } => write!(f, "RetryWorkers(max_retries={})", max_retries),
        }
    }
}

/// Per-phase timing data: tracks when a phase started and optionally when it ended.
#[derive(Clone)]
pub struct PhaseTimings {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
}

impl Debug for PhaseTimings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Display for PhaseTimings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.end_time {
            Some(end) => {
                let duration = end.signed_duration_since(self.start_time);
                write!(f, "{}ms [{} - {}]", duration.num_milliseconds(), self.start_time, end)
            }
            None => write!(f, "in progress [started {}]", self.start_time),
        }
    }
}

#[derive(Debug)]
pub struct Job {
    pub job_id: JobId,
    pub hash_id: String,
    pub phase_timings: HashMap<JobPhase, PhaseTimings>,
    pub task_received_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub state: JobState,
    pub data_id: DataId,
    pub inputs_mode: InputsModeDto,
    pub hints_mode: HintsModeDto,
    pub compute_capacity: ComputeCapacity,
    pub minimal_compute_capacity: ComputeCapacity,
    pub workers: Vec<WorkerId>,
    pub agg_worker_id: Option<WorkerId>,
    pub partitions: Vec<Vec<u32>>,
    pub results: HashMap<JobPhase, HashMap<WorkerId, JobResult>>,
    pub challenges: Option<Vec<ContributionsInfo>>,
    pub witness_info: Option<WitnessInfo>,
    pub execution_mode: JobExecutionMode,
    pub proof: Option<Proof>,
    pub executed_steps: Option<u64>,
    pub instances: Option<u64>,
    pub metadata: BTreeMap<String, String>,
    pub execution_only: bool,
    pub proof_type: ProofKind,
    /// Aggregation task currently in-flight to the recurser (sent, not yet acked).
    /// Re-sent verbatim if the recurser reconnects before returning its result.
    pub agg_task_inflight: Option<PendingAggTask>,
    pub agg_task_queue: VecDeque<PendingAggTask>,
}

impl Job {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: JobId,
        data_id: DataId,
        hash_id: String,
        inputs_mode: InputsModeDto,
        hints_mode: HintsModeDto,
        compute_capacity: ComputeCapacity,
        minimal_compute_capacity: ComputeCapacity,
        selected_workers: Vec<WorkerId>,
        partitions: Vec<Vec<u32>>,
        execution_mode: JobExecutionMode,
        metadata: BTreeMap<String, String>,
        execution_only: bool,
        proof_type: ProofKind,
    ) -> Self {
        Self {
            job_id,
            hash_id,
            phase_timings: HashMap::new(),
            duration_ms: None,
            terminated_at: None,
            state: JobState::Created,
            data_id,
            inputs_mode,
            hints_mode,
            compute_capacity,
            minimal_compute_capacity,
            workers: selected_workers,
            agg_worker_id: None,
            partitions,
            results: HashMap::new(),
            task_received_time: None,
            challenges: None,
            witness_info: None,
            execution_mode,
            proof: None,
            executed_steps: None,
            instances: None,
            metadata,
            execution_only,
            proof_type,
            agg_task_inflight: None,
            agg_task_queue: VecDeque::new(),
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn change_state(&mut self, new_state: JobState) {
        // Validate transition. Failed and Cancelled are always reachable (abort from any state).
        let valid = matches!(
            (&self.state, &new_state),
            (_, JobState::Failed)
                | (_, JobState::Cancelled)
                | (JobState::Created, JobState::Running(_))
                | (JobState::Running(_), JobState::Running(_))
                | (JobState::Running(_), JobState::Completed)
        );

        if !valid {
            error!(
                "Invalid job state transition for {}: {} -> {}",
                self.job_id, self.state, new_state
            );
            return;
        }

        // Record end_time for the phase we're leaving
        if let JobState::Running(ref current_phase) = self.state {
            if let Some(timings) = self.phase_timings.get_mut(current_phase) {
                timings.end_time = Some(Utc::now());
            }
        }

        self.state = new_state.clone();

        match new_state {
            JobState::Running(phase) => {
                let previous = self
                    .phase_timings
                    .insert(phase.clone(), PhaseTimings { start_time: Utc::now(), end_time: None });
                if previous.is_some() {
                    error!("Start time for phase {:?} was already set", phase);
                }
            }
            JobState::Completed | JobState::Failed | JobState::Cancelled => {
                let now = Utc::now();
                let earliest_start = self.phase_timings.values().map(|t| t.start_time).min();
                if let Some(start_time) = earliest_start {
                    let duration = now.signed_duration_since(start_time);
                    self.duration_ms = Some(duration.num_milliseconds() as u64);
                }
                self.terminated_at = Some(now);
            }
            _ => {}
        }
    }

    /// Returns the start time for a given phase, if recorded.
    pub fn phase_start_time(&self, phase: &JobPhase) -> Option<DateTime<Utc>> {
        self.phase_timings.get(phase).map(|t| t.start_time)
    }

    pub fn state(&self) -> &JobState {
        &self.state
    }

    pub fn cleanup(&mut self) {
        self.partitions.clear();
        self.results.clear();
        self.phase_timings.clear();
        self.challenges = None;
        self.agg_task_inflight = None;
        self.agg_task_queue.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Created,
    Running(JobPhase),
    Completed,
    Failed,
    Cancelled,
}

impl JobState {
    pub fn is_resolved(&self) -> bool {
        matches!(self, JobState::Failed | JobState::Completed | JobState::Cancelled)
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobState::Created => write!(f, "Created"),
            JobState::Running(phase) => write!(f, "Running ({:?})", phase),
            JobState::Completed => write!(f, "Completed"),
            JobState::Failed => write!(f, "Failed"),
            JobState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AggProofData {
    pub worker_idx: u32,
    pub airgroup_id: u64,
    pub values: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct PendingAggTask {
    pub proofs: Vec<AggProofData>,
    pub all_done: bool,
    pub proof_type: ProofKind,
}

#[derive(Debug, Clone)]
pub struct ContributionsResult {
    pub challenges: Vec<ContributionsInfo>,
    pub witness_info: WitnessInfo,
    pub zisk_executor_time: ZiskExecutorTime,
    pub task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    pub instances: u64,
    pub cost_per_type: StatsCostPerType,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub instances: u64,
    pub executed_steps: u64,
    pub zisk_executor_time: ZiskExecutorTime,
    pub task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    pub public_outputs: Vec<u8>,
    pub cost_per_type: StatsCostPerType,
    pub plan: Vec<zisk_common::AirInstanceCount>,
}

#[derive(Debug, Clone)]
pub enum JobResultData {
    Execution(ExecutionResult),
    Challenges(ContributionsResult),
    AggProofs(Vec<AggProofData>),
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub success: bool,
    pub data: JobResultData,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DataCtx {
    pub data_id: DataId,
    pub input_source: InputSourceDto,
    pub hints_source: HintsSourceDto,
}

#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, BorshSerialize, BorshDeserialize)]
pub enum JobPhase {
    Execution,
    Contributions,
    Prove,
    Recurse,
    ContributionsInputsStream,
    ContributionsHintsStream,
}

impl TryFrom<u8> for JobPhase {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(JobPhase::Execution),
            1 => Ok(JobPhase::Contributions),
            2 => Ok(JobPhase::Prove),
            3 => Ok(JobPhase::Recurse),
            4 => Ok(JobPhase::ContributionsInputsStream),
            5 => Ok(JobPhase::ContributionsHintsStream),
            _ => Err(anyhow::anyhow!("Invalid JobPhase byte: {}", value)),
        }
    }
}

impl fmt::Display for JobPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobPhase::Execution => write!(f, "Execution"),
            JobPhase::Contributions => write!(f, "Contributions"),
            JobPhase::Prove => write!(f, "Prove"),
            JobPhase::Recurse => write!(f, "Recurse"),
            JobPhase::ContributionsInputsStream => write!(f, "ContributionsInputsStream"),
            JobPhase::ContributionsHintsStream => write!(f, "ContributionsHintsStream"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerAllocationDto {
    pub range: Range<u32>,
}

#[derive(Debug, Clone)]
pub struct AggregationParams {
    pub agg_proofs: Vec<AggProofData>,
    pub last_proof: bool,
    pub final_proof: bool,
    pub proof_type: ProofKind,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PartitionInfo {
    pub total_compute_units: usize,
    pub allocation: Vec<u32>,
    pub worker_idx: usize,
}

/// Message structures for MPI broadcast to ensure type safety
#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct ContributionsMessage {
    pub job_id: JobId,
    pub hash_id: String,
    pub phase_inputs: ProvePhaseInputs,
    pub options: ProofOptions,
    pub input_source: InputSourceDto,
    pub hints_source: HintsSourceDto,
    pub partition_info: PartitionInfo,
}

#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct ProveMessage {
    pub job_id: JobId,
    pub phase_inputs: ProvePhaseInputs,
    pub options: ProofOptions,
}

#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct StreamMessage {
    pub data: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job() -> Job {
        Job::new(
            JobId::new(),
            Default::default(),
            String::new(),
            crate::InputsModeDto::InputsNone,
            crate::HintsModeDto::HintsNone,
            ComputeCapacity::from(1u32),
            ComputeCapacity::from(1u32),
            vec![],
            vec![],
            JobExecutionMode::Standard,
            BTreeMap::new(),
            false,
            crate::ProofKind::VadcopFinal,
        )
    }

    #[test]
    fn test_valid_state_transitions() {
        let mut job = make_job();
        assert_eq!(job.state, JobState::Created);

        // Created → Running(Contributions)
        job.change_state(JobState::Running(JobPhase::Contributions));
        assert_eq!(job.state, JobState::Running(JobPhase::Contributions));

        // Running → Running (phase change)
        job.change_state(JobState::Running(JobPhase::Prove));
        assert_eq!(job.state, JobState::Running(JobPhase::Prove));

        // Running → Completed
        job.change_state(JobState::Completed);
        assert_eq!(job.state, JobState::Completed);
    }

    #[test]
    fn test_invalid_transition_created_to_completed() {
        let mut job = make_job();

        // Created → Completed is invalid
        job.change_state(JobState::Completed);
        assert_eq!(job.state, JobState::Created); // state unchanged
    }

    #[test]
    fn test_invalid_transition_completed_to_running() {
        let mut job = make_job();
        job.change_state(JobState::Running(JobPhase::Contributions));
        job.change_state(JobState::Completed);

        // Completed → Running is invalid
        job.change_state(JobState::Running(JobPhase::Prove));
        assert_eq!(job.state, JobState::Completed); // state unchanged
    }

    #[test]
    fn test_failed_always_reachable() {
        let mut job = make_job();

        // Created → Failed
        job.change_state(JobState::Failed);
        assert_eq!(job.state, JobState::Failed);

        // Another job: Running → Failed
        let mut job2 = make_job();
        job2.change_state(JobState::Running(JobPhase::Prove));
        job2.change_state(JobState::Failed);
        assert_eq!(job2.state, JobState::Failed);
    }

    #[test]
    fn test_duplicate_phase_start_time_does_not_crash() {
        let mut job = make_job();

        // First time: insert Contributions start time
        job.change_state(JobState::Running(JobPhase::Contributions));
        assert!(job.phase_start_time(&JobPhase::Contributions).is_some());

        // Manually re-insert to simulate the error path
        // (normally prevented by state machine, but we test the error! path)
        let original_time = job.phase_start_time(&JobPhase::Contributions).unwrap();
        job.phase_timings.insert(
            JobPhase::Contributions,
            PhaseTimings { start_time: Utc::now(), end_time: None },
        );

        // The job should still be functional — no panic
        job.change_state(JobState::Running(JobPhase::Prove));
        assert!(job.phase_start_time(&JobPhase::Prove).is_some());

        // Verify first phase was overwritten (not panicked)
        assert_ne!(job.phase_start_time(&JobPhase::Contributions).unwrap(), original_time);
    }

    #[test]
    fn test_duration_computed_on_completion() {
        let mut job = make_job();
        job.change_state(JobState::Running(JobPhase::Contributions));
        job.change_state(JobState::Completed);

        // duration_ms should be set (very small since no real work)
        assert!(job.duration_ms.is_some());
    }

    #[test]
    fn test_cleanup_clears_phase_timings() {
        let mut job = make_job();
        job.change_state(JobState::Running(JobPhase::Contributions));
        assert!(!job.phase_timings.is_empty());

        job.cleanup();
        assert!(job.phase_timings.is_empty());
    }

    #[test]
    fn test_phase_end_time_recorded_on_transition() {
        let mut job = make_job();
        job.change_state(JobState::Running(JobPhase::Contributions));

        // End time not set yet
        assert!(job.phase_timings.get(&JobPhase::Contributions).unwrap().end_time.is_none());

        // Transition to next phase records end_time on previous
        job.change_state(JobState::Running(JobPhase::Prove));
        assert!(job.phase_timings.get(&JobPhase::Contributions).unwrap().end_time.is_some());
    }
}
