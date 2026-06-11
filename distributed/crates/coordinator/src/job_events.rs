use zisk_cluster_common::JobPhase;

/// Events broadcast on the per-job channel as the job transitions through states.
#[derive(Debug, Clone)]
pub enum CoordinatorJobEvent {
    Queued,
    Started,
    Progress(JobPhase),
    WaitingForInput,
    Completed(CoordinatorJobResult),
    Failed(String),
    Cancelled,
}

/// The result payload carried by a `Completed` event.
#[derive(Debug, Clone)]
pub enum CoordinatorJobResult {
    Setup { vk: Vec<u8>, hash_mode: String },
    Prove { proof_bytes: Vec<u8>, stats: CoordinatorExecutionStats },
    Execute { stats: CoordinatorExecutionStats, public_outputs: Vec<u8> },
    Wrap { proof_bytes: Vec<u8> },
    SetupAggregationProgram { vk: Vec<u8>, hash_mode: String },
    AggregateProofs { proof_bytes: Vec<u8> },
}

/// Execution statistics forwarded to the coordinator on job completion.
#[derive(Debug, Clone, Default)]
pub struct CoordinatorExecutionStats {
    pub steps: u64,
    pub duration_nanos: u64,
    pub main_cost: u64,
    pub opcode_cost: u64,
    pub memory_cost: u64,
    pub precompile_cost: u64,
    pub tables_cost: u64,
    pub other_cost: u64,
}
