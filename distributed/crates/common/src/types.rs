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
        write!(f, "JobId({})", self.0)
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
        write!(f, "BlockId({})", self.0)
    }
}

/// Prover ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProverId(String);

impl Default for ProverId {
    fn default() -> Self {
        Self::new()
    }
}

impl ProverId {
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

impl From<String> for ProverId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<ProverId> for String {
    fn from(prover_id: ProverId) -> Self {
        prover_id.0
    }
}

impl std::fmt::Display for ProverId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProverId({})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProverState {
    Disconnected,
    Connecting,
    Idle,
    Computing(JobPhase),
    Error,
}

impl Display for ProverState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state_str = match self {
            ProverState::Disconnected => "Disconnected",
            ProverState::Connecting => "Connecting",
            ProverState::Idle => "Idle",
            ProverState::Computing(phase) => return write!(f, "Computing({})", phase),
            ProverState::Error => "Error",
        };
        write!(f, "{}", state_str)
    }
}

/// Compute capacity for provers
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
        write!(f, "{} CU", self.compute_units)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobExecutionMode {
    Standard,        // the normal mode
    Simulating(u32), // simulation mode where we simulate N nodes but only use one prover
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
    pub start_time: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub state: JobState,
    pub block: BlockContext,
    pub compute_units: ComputeCapacity,
    pub provers: Vec<ProverId>,
    pub agg_prover: Option<ProverId>,
    pub partitions: Vec<Vec<u32>>,
    pub results: HashMap<JobPhase, HashMap<ProverId, JobResult>>,
    pub stats: HashMap<JobPhase, JobStats>,
    pub challenges: Option<Vec<ContributionsInfo>>,
    pub execution_mode: JobExecutionMode,
}

impl Job {
    pub fn new(
        block_id: BlockId,
        input_path: PathBuf,
        compute_units: ComputeCapacity,
        selected_provers: Vec<ProverId>,
        partitions: Vec<Vec<u32>>,
        execution_mode: JobExecutionMode,
    ) -> Self {
        Self {
            job_id: JobId::new(),
            start_time: Utc::now(),
            duration_ms: None,
            state: JobState::Created,
            block: BlockContext { block_id, input_path },
            compute_units,
            provers: selected_provers,
            agg_prover: None,
            partitions,
            results: HashMap::new(),
            stats: HashMap::new(),
            challenges: None,
            execution_mode,
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

        if matches!(new_state, JobState::Completed | JobState::Failed) {
            let end_time = Utc::now();
            let duration = end_time.signed_duration_since(self.start_time);
            self.duration_ms = Some(duration.num_milliseconds() as u64);
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
}

#[derive(Debug, Clone)]
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
pub struct ProverAllocationDto {
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
