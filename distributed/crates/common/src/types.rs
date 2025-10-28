//! Core Domain Types for Distributed Proving System
//!
//! This module defines the fundamental domain types and business entities used throughout
//! the distributed proving system. These types form the core vocabulary of the system,
//! providing type safety, semantic clarity, and domain-driven design principles.

use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use proofman::ContributionsInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Debug, Display},
    ops::Range,
    path::PathBuf,
};
use tracing::error;

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

/// Block ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BlockId(String);

impl Default for BlockId {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockId {
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

impl From<String> for BlockId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<BlockId> for String {
    fn from(block_id: BlockId) -> Self {
        block_id.0
    }
}

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 8 {
            write!(f, "BlockId({:.8}…)", self.0)
        } else {
            write!(f, "BlockId({})", self.0)
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
    Idle,
    Computing((JobId, JobPhase)),
    Error,
}

impl Display for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = match self {
            WorkerState::Disconnected => "Disconnected",
            WorkerState::Connecting => "Connecting",
            WorkerState::Idle => "Idle",
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

#[derive(Clone)]
pub struct JobStats {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl Display for JobStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start_time = self.start_time.unwrap();
        let end_time = self.end_time.unwrap();
        let duration = end_time.signed_duration_since(start_time);

        write!(f, "Duration: {}ms", duration.num_milliseconds())
    }
}

impl Debug for JobStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start_time = self.start_time.unwrap();
        let end_time = self.end_time.unwrap();
        let duration = end_time.signed_duration_since(start_time);

        write!(f, "Duration: {}ms", duration.num_milliseconds())
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub job_id: JobId,
    pub start_times: HashMap<JobPhase, DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub state: JobState,
    pub block: BlockContext,
    pub compute_capacity: ComputeCapacity,
    pub workers: Vec<WorkerId>,
    pub agg_worker_id: Option<WorkerId>,
    pub partitions: Vec<Vec<u32>>,
    pub results: HashMap<JobPhase, HashMap<WorkerId, JobResult>>,
    pub stats: HashMap<JobPhase, JobStats>,
    pub challenges: Option<Vec<ContributionsInfo>>,
    pub execution_mode: JobExecutionMode,
    pub final_proof: Option<Vec<u64>>,
    pub executed_steps: Option<u64>,
}

impl Job {
    pub fn new(
        block_id: BlockId,
        input_path: PathBuf,
        compute_capacity: ComputeCapacity,
        selected_workers: Vec<WorkerId>,
        partitions: Vec<Vec<u32>>,
        execution_mode: JobExecutionMode,
    ) -> Self {
        Self {
            job_id: JobId::new(),
            start_times: HashMap::new(),
            duration_ms: None,
            state: JobState::Created,
            block: BlockContext { block_id, input_path },
            compute_capacity,
            workers: selected_workers,
            agg_worker_id: None,
            partitions,
            results: HashMap::new(),
            stats: HashMap::new(),
            challenges: None,
            execution_mode,
            final_proof: None,
            executed_steps: None,
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn change_state(&mut self, new_state: JobState) {
        if let JobState::Running(current_state) = &self.state {
            self.add_end_time(current_state.clone());
        }

        self.state = new_state.clone();

        if let JobState::Running(new_phase) = &new_state {
            self.add_start_time(new_phase.clone());
        }

        match new_state {
            JobState::Running(phase) => {
                let previous = self.start_times.insert(phase.clone(), Utc::now());
                if previous.is_some() {
                    error!("Start time for phase {:?} was already set", phase);
                }
            }
            JobState::Completed | JobState::Failed => {
                let end_time = Utc::now();
                if let Some(start_time) = self.start_times.get(&JobPhase::Contributions) {
                    let duration = end_time.signed_duration_since(*start_time);
                    self.duration_ms = Some(duration.num_milliseconds() as u64);
                }
            }
            _ => {}
        }
    }

    fn add_start_time(&mut self, job_phase: JobPhase) {
        match self.stats.get_mut(&job_phase) {
            Some(_) => {
                unreachable!("Start time added twice for the same phase");
            }
            None => {
                self.stats
                    .insert(job_phase, JobStats { start_time: Some(Utc::now()), end_time: None });
            }
        }
    }

    fn add_end_time(&mut self, job_phase: JobPhase) {
        match self.stats.get_mut(&job_phase) {
            Some(existing_stats) => {
                existing_stats.end_time = Some(Utc::now());
            }
            None => unreachable!("End time added without start time"),
        }
    }

    pub fn state(&self) -> &JobState {
        &self.state
    }

    pub fn cleanup(&mut self) {
        self.partitions.clear();
        self.results.clear();
        self.stats.clear();
        self.challenges = None;
        self.final_proof = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Created,
    Running(JobPhase),
    Completed,
    Failed,
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobState::Created => write!(f, "Created"),
            JobState::Running(phase) => write!(f, "Running ({:?})", phase),
            JobState::Completed => write!(f, "Completed"),
            JobState::Failed => write!(f, "Failed"),
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
pub enum JobResultData {
    Challenges(Vec<ContributionsInfo>),
    AggProofs(Vec<AggProofData>),
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub success: bool,
    pub data: JobResultData,
}

#[derive(Debug, Clone)]
pub struct BlockContext {
    pub block_id: BlockId,
    pub input_path: PathBuf,
}

#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, BorshSerialize, BorshDeserialize)]
pub enum JobPhase {
    Contributions,
    Prove,
    Aggregate,
}

impl TryFrom<u8> for JobPhase {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(JobPhase::Contributions),
            1 => Ok(JobPhase::Prove),
            2 => Ok(JobPhase::Aggregate),
            _ => Err(anyhow::anyhow!("Invalid JobPhase byte: {}", value)),
        }
    }
}

impl fmt::Display for JobPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobPhase::Contributions => write!(f, "Contributions"),
            JobPhase::Prove => write!(f, "Prove"),
            JobPhase::Aggregate => write!(f, "Aggregate"),
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
    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,
    pub verify_proofs: bool,
    pub save_proofs: bool,
    pub test_mode: bool,
    pub output_dir_path: PathBuf,
    pub minimal_memory: bool,
}
