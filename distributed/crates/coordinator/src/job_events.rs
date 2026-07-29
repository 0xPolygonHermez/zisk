use zisk_cluster_common::JobPhase;
use zisk_common::{AirInstanceCount, ZiskExecutorTime};

/// Events broadcast on the per-job channel as the job transitions through states.
#[derive(Debug, Clone)]
pub enum CoordinatorJobEvent {
    /// Job accepted and waiting to start.
    Queued,
    /// Job has started executing.
    Started,
    /// Job advanced to a new phase.
    Progress(JobPhase),
    /// Job is paused awaiting streamed input.
    WaitingForInput,
    /// Job finished successfully, carrying its result.
    Completed(CoordinatorJobResult),
    /// Job failed, with an error message.
    Failed(String),
    /// Job was cancelled.
    Cancelled,
}

/// The result payload carried by a `Completed` event.
#[derive(Debug, Clone)]
pub enum CoordinatorJobResult {
    /// Guest-program setup result.
    Setup {
        /// The program verification key.
        vk: Vec<u8>,
        /// The hash mode the key was generated with.
        hash_mode: String,
    },
    /// Proof-generation result.
    Prove {
        /// The serialized proof.
        proof_bytes: Vec<u8>,
        /// Execution statistics.
        stats: CoordinatorExecutionStats,
    },
    /// Execute-only result (no proof).
    Execute {
        /// Execution statistics.
        stats: CoordinatorExecutionStats,
        /// The program's public outputs.
        public_outputs: Vec<u8>,
    },
    /// Proof-wrapping result.
    Wrap {
        /// The wrapped proof.
        proof_bytes: Vec<u8>,
    },
    /// Aggregation-program setup result.
    SetupAggregationProgram {
        /// The aggregation-program verification key.
        vk: Vec<u8>,
        /// The hash mode the key was generated with.
        hash_mode: String,
    },
    /// Proof-aggregation result.
    AggregateProofs {
        /// The aggregated proof.
        proof_bytes: Vec<u8>,
    },
}

/// Execution statistics forwarded to the coordinator on job completion.
#[derive(Debug, Clone, Default)]
pub struct CoordinatorExecutionStats {
    /// Number of executed steps.
    pub steps: u64,
    /// Total execution duration, in nanoseconds.
    pub duration_nanos: u64,
    /// Cost attributed to the main state machine.
    pub main_cost: u64,
    /// Cost attributed to opcode execution.
    pub opcode_cost: u64,
    /// Cost attributed to memory operations.
    pub memory_cost: u64,
    /// Cost attributed to precompiles.
    pub precompile_cost: u64,
    /// Cost attributed to lookup tables.
    pub tables_cost: u64,
    /// Cost not attributed to any of the above categories.
    pub other_cost: u64,
    /// Per-phase executor timing breakdown.
    pub executor_time: ZiskExecutorTime,
    /// Per-AIR instance plan.
    pub plan: Vec<AirInstanceCount>,
}
