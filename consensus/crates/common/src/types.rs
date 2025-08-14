use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Range, path::PathBuf};

/// Job ID wrapper for type safety
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    pub state: JobState,
    pub block: BlockContext,
    pub compute_units: u32,
    pub provers: Vec<ProverId>,
    pub partitions: Vec<Range<u32>>,
    pub results: HashMap<JobPhase, HashMap<ProverId, JobResult>>,
    pub challenges: Option<Vec<Vec<u64>>>,
}

#[derive(Debug, Clone)]
pub enum JobState {
    Idle,
    Running(JobPhase),
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct RowData {
    pub values: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub success: bool,
    pub data: Vec<RowData>,
}

#[derive(Debug, Clone)]
pub struct BlockContext {
    pub block_id: BlockId,
    pub input_path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum JobPhase {
    Phase1,
    Phase2,
    PhaseAggregation,
}

impl JobPhase {
    pub fn as_string(&self) -> String {
        match self {
            JobPhase::Phase1 => "Phase1".to_string(),
            JobPhase::Phase2 => "Phase2".to_string(),
            JobPhase::PhaseAggregation => "PhaseAggregation".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProverAllocationDto {
    pub range: Range<u32>,
}
