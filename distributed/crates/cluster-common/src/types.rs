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
    /// Generate a fresh random job id.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Clone the id into an owned `String`.
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
    /// Generate a fresh random data id.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Clone the id into an owned `String`.
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
    /// Generate a fresh random worker id.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Clone the id into an owned `String`.
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

/// Lifecycle state of a worker as tracked by the coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState {
    /// Not connected.
    Disconnected,
    /// Connection in progress.
    Connecting,
    /// Connected but no setup done yet. Not eligible for job assignment.
    Idle,
    /// Running setup (guest program load). Not eligible for job assignment.
    SettingUp,
    /// Setup complete. Eligible for job assignment.
    Ready,
    /// Computing the given phase of the given job.
    Computing((JobId, JobPhase)),
    /// In an error state.
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
    /// Number of compute units.
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

/// How a job is executed across workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobExecutionMode {
    /// Normal mode: real workers.
    Standard,
    /// Simulation mode: simulate N workers using only one physical worker.
    Simulating(u32),
}

impl JobExecutionMode {
    /// Whether this is [`Simulating`](Self::Simulating) mode.
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
    /// When the phase started.
    pub start_time: DateTime<Utc>,
    /// When the phase ended, if it has.
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

/// The coordinator's full state for a single job across its lifecycle.
#[derive(Debug)]
pub struct Job {
    /// The job's id.
    pub job_id: JobId,
    /// Content hash of the guest program being proven.
    pub hash_id: String,
    /// Start/end timings per phase.
    pub phase_timings: HashMap<JobPhase, PhaseTimings>,
    /// When the job request was received.
    pub task_received_time: Option<DateTime<Utc>>,
    /// Total duration once terminal, in milliseconds.
    pub duration_ms: Option<u64>,
    /// When the job reached a terminal state.
    pub terminated_at: Option<DateTime<Utc>>,
    /// Current job state.
    pub state: JobState,
    /// Id of the input/hints data context.
    pub data_id: DataId,
    /// How inputs are supplied.
    pub inputs_mode: InputsModeDto,
    /// How hints are supplied.
    pub hints_mode: HintsModeDto,
    /// Requested compute capacity.
    pub compute_capacity: ComputeCapacity,
    /// Minimum acceptable compute capacity.
    pub minimal_compute_capacity: ComputeCapacity,
    /// Workers assigned to the job.
    pub workers: Vec<WorkerId>,
    /// Worker handling aggregation, if assigned.
    pub agg_worker_id: Option<WorkerId>,
    /// Per-worker compute-unit partitions.
    pub partitions: Vec<Vec<u32>>,
    /// Per-phase, per-worker results.
    pub results: HashMap<JobPhase, HashMap<WorkerId, JobResult>>,
    /// Collected contribution challenges, once available.
    pub challenges: Option<Vec<ContributionsInfo>>,
    /// Witness metadata, once available.
    pub witness_info: Option<WitnessInfo>,
    /// Execution mode (standard or simulating).
    pub execution_mode: JobExecutionMode,
    /// The final proof, once produced.
    pub proof: Option<Proof>,
    /// Total executed steps, once known.
    pub executed_steps: Option<u64>,
    /// Number of AIR instances, once known.
    pub instances: Option<u64>,
    /// Arbitrary client-supplied metadata.
    pub metadata: Option<BTreeMap<String, String>>,
    /// Whether this is an execute-only job (no proof).
    pub execution_only: bool,
    /// The kind of proof requested.
    pub proof_type: ProofKind,
    /// Aggregation task currently in-flight to the recurser (sent, not yet acked).
    /// Re-sent verbatim if the recurser reconnects before returning its result.
    pub agg_task_inflight: Option<PendingAggTask>,
    /// Queued aggregation tasks awaiting dispatch.
    pub agg_task_queue: VecDeque<PendingAggTask>,
}

impl Job {
    /// Create a new job in the `Created` state from its submission parameters.
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
        metadata: Option<BTreeMap<String, String>>,
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

    /// The job's id.
    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    /// Transition the job to `new_state`, validating the transition and
    /// recording phase timings and terminal duration. Invalid transitions are
    /// logged and ignored.
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

    /// The job's current state.
    pub fn state(&self) -> &JobState {
        &self.state
    }

    /// Release per-job working memory (partitions, results, timings, challenges,
    /// and aggregation queues) once the job is terminal.
    pub fn cleanup(&mut self) {
        self.partitions.clear();
        self.results.clear();
        self.phase_timings.clear();
        self.challenges = None;
        self.agg_task_inflight = None;
        self.agg_task_queue.clear();
    }
}

/// Coarse job state (the phase lives inside `Running`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Created, not yet running.
    Created,
    /// Running the given phase.
    Running(JobPhase),
    /// Finished successfully.
    Completed,
    /// Finished with a failure.
    Failed,
    /// Cancelled.
    Cancelled,
}

impl JobState {
    /// Whether this is a resolved (terminal) state.
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

/// A single airgroup's partial proof produced by a worker.
#[derive(Debug, Clone)]
pub struct AggProofData {
    /// Index of the producing worker.
    pub worker_idx: u32,
    /// The airgroup this proof belongs to.
    pub airgroup_id: u64,
    /// The proof field-element values.
    pub values: Vec<u64>,
}

/// An aggregation task queued or in-flight to the recurser.
#[derive(Debug, Clone)]
pub struct PendingAggTask {
    /// The partial proofs to aggregate.
    pub proofs: Vec<AggProofData>,
    /// Whether this is the final aggregation step.
    pub all_done: bool,
    /// The kind of proof being produced.
    pub proof_type: ProofKind,
}

/// Result of the contribution phase for one worker.
#[derive(Debug, Clone)]
pub struct ContributionsResult {
    /// The contribution challenges.
    pub challenges: Vec<ContributionsInfo>,
    /// Witness metadata.
    pub witness_info: WitnessInfo,
    /// Executor timing.
    pub zisk_executor_time: ZiskExecutorTime,
    /// When the originating task was received.
    pub task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of AIR instances.
    pub instances: u64,
    /// Per-type cost breakdown.
    pub cost_per_type: StatsCostPerType,
}

/// Result of an execute-only run for one worker.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Number of AIR instances.
    pub instances: u64,
    /// Steps executed.
    pub executed_steps: u64,
    /// Executor timing.
    pub zisk_executor_time: ZiskExecutorTime,
    /// When the originating task was received.
    pub task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    /// The program's public outputs.
    pub public_outputs: Vec<u8>,
    /// Per-type cost breakdown.
    pub cost_per_type: StatsCostPerType,
    /// Per-AIR instance plan.
    pub plan: Vec<zisk_common::AirInstanceCount>,
}

/// The payload of a per-worker job result, by phase.
#[derive(Debug, Clone)]
pub enum JobResultData {
    /// Execute-only result.
    Execution(ExecutionResult),
    /// Contribution-phase result.
    Challenges(ContributionsResult),
    /// Partial proofs from the prove phase.
    AggProofs(Vec<AggProofData>),
}

/// A worker's result for one phase of a job.
#[derive(Debug, Clone)]
pub struct JobResult {
    /// Whether the phase succeeded for this worker.
    pub success: bool,
    /// The result payload.
    pub data: JobResultData,
    /// When the result was recorded.
    pub end_time: DateTime<Utc>,
}

/// The input/hints data context for a job.
#[derive(Debug, Clone)]
pub struct DataCtx {
    /// Id of this data context.
    pub data_id: DataId,
    /// Where the job's input comes from.
    pub input_source: InputSourceDto,
    /// Where the job's hints come from.
    pub hints_source: HintsSourceDto,
}

/// A phase of a job's execution. The `#[repr(u8)]` order is wire-significant
/// (used by [`TryFrom<u8>`]).
#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, BorshSerialize, BorshDeserialize)]
pub enum JobPhase {
    /// Execute-only phase.
    Execution,
    /// Contribution-gathering phase.
    Contributions,
    /// Proof-generation phase.
    Prove,
    /// Recursion/aggregation phase.
    Recurse,
    /// Streaming of contribution inputs.
    ContributionsInputsStream,
    /// Streaming of contribution hints.
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

/// A worker's assigned compute-unit range within a job.
#[derive(Debug, Clone)]
pub struct WorkerAllocationDto {
    /// The compute-unit index range assigned to the worker.
    pub range: Range<u32>,
}

/// Parameters for an aggregation step.
#[derive(Debug, Clone)]
pub struct AggregationParams {
    /// Partial proofs to aggregate.
    pub agg_proofs: Vec<AggProofData>,
    /// Whether this is the last proof in the current round.
    pub last_proof: bool,
    /// Whether this produces the final job proof.
    pub final_proof: bool,
    /// The kind of proof being produced.
    pub proof_type: ProofKind,
}

/// A worker's partition of a job's total work.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct PartitionInfo {
    /// Total compute units across the job.
    pub total_compute_units: usize,
    /// Per-worker compute-unit allocation.
    pub allocation: Vec<u32>,
    /// This worker's index into `allocation`.
    pub worker_idx: usize,
}

/// MPI broadcast message carrying a contribution/execution task to peer ranks.
#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct ContributionsMessage {
    /// The job id.
    pub job_id: JobId,
    /// Content hash of the guest program.
    pub hash_id: String,
    /// Proving-phase inputs.
    pub phase_inputs: ProvePhaseInputs,
    /// Proof options.
    pub options: ProofOptions,
    /// Where the input comes from.
    pub input_source: InputSourceDto,
    /// Where the hints come from.
    pub hints_source: HintsSourceDto,
    /// This rank's partition.
    pub partition_info: PartitionInfo,
}

/// MPI broadcast message carrying a prove task to peer ranks.
#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct ProveMessage {
    /// The job id.
    pub job_id: JobId,
    /// Proving-phase inputs (challenges).
    pub phase_inputs: ProvePhaseInputs,
    /// Proof options.
    pub options: ProofOptions,
}

/// MPI broadcast message carrying a chunk of streamed data.
#[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct StreamMessage {
    /// The streamed words.
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
            None,
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
