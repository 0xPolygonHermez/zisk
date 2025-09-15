use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use proofman::ContributionsInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
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

impl std::fmt::Display for ComputeCapacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} CU", self.compute_units)
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub job_id: JobId,
    pub start_time: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub state: JobState,
    pub block: BlockContext,
    pub compute_units: u32,
    pub provers: Vec<ProverId>,
    pub partitions: Vec<Vec<u32>>,
    pub results: HashMap<JobPhase, HashMap<ProverId, JobResult>>,
    pub challenges: Option<Vec<ContributionsInfo>>,
}

#[derive(Debug, Clone)]
pub enum JobState {
    Running(JobPhase),
    Completed,
    Failed,
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
