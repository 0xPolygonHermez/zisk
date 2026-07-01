use std::fmt;
use std::time::Instant;

/// Type representing a chunk identifier.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId(pub usize);

impl ChunkId {
    /// Creates a new `ChunkId` with the given identifier.
    pub const fn new(id: usize) -> Self {
        ChunkId(id)
    }

    /// Returns the underlying `usize` identifier of the `ChunkId`.
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl PartialEq<usize> for ChunkId {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl From<ChunkId> for usize {
    fn from(id: ChunkId) -> Self {
        id.0
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type representing a chunk identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentId(pub usize);

impl SegmentId {
    /// Creates a new `SegmentId` with the given identifier.
    pub const fn new(id: usize) -> Self {
        SegmentId(id)
    }

    /// Returns the underlying `usize` identifier of the `SegmentId`.
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl PartialEq<usize> for SegmentId {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl From<SegmentId> for usize {
    fn from(id: SegmentId) -> Self {
        id.0
    }
}

impl fmt::Display for SegmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type representing different statistics categories.
pub enum StatsType {
    /// Main execution stats.
    Main,
    /// Memory-related operations and their stats.
    Memory,
    /// Opcode execution stats.
    Opcodes,
    /// Precompiled function execution stats.
    Precompiled,
    /// Table-related operations and their stats.
    Tables,
    /// Other miscellaneous stats.
    Other,
}

/// Struct to hold cost breakdown by different types of operations.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatsCostPerType {
    /// Cost associated with the main state machine.
    pub main_cost: u64,
    /// Cost associated with the non-main state machines (neither main nor precompiled).
    pub opcode_cost: u64,
    /// Cost associated with memory-related state machines.
    pub memory_cost: u64,
    /// Cost associated with precompiled state machines.
    pub precompile_cost: u64,
    /// Cost associated with table-related operations.
    pub tables_cost: u64,
    /// Cost associated with other miscellaneous operations.
    pub other_cost: u64,
}

impl StatsCostPerType {
    /// Calculates the total cost by summing all individual cost categories.
    pub fn total_cost(&self) -> u64 {
        self.main_cost
            + self.opcode_cost
            + self.memory_cost
            + self.precompile_cost
            + self.tables_cost
            + self.other_cost
    }

    /// Adds the given cost to the appropriate category based on the `StatsType`.
    pub fn add_cost(&mut self, stats_type: StatsType, cost: u64) {
        match stats_type {
            StatsType::Main => self.main_cost += cost,
            StatsType::Opcodes => self.opcode_cost += cost,
            StatsType::Memory => self.memory_cost += cost,
            StatsType::Precompiled => self.precompile_cost += cost,
            StatsType::Tables => self.tables_cost += cost,
            StatsType::Other => self.other_cost += cost,
        }
    }
}

impl fmt::Display for StatsCostPerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.total_cost();
        if total == 0 {
            return write!(f, "total=0");
        }

        let mut parts = Vec::new();

        let pct = (self.main_cost as f64 / total as f64) * 100.0;
        parts.push(format!("main={} ({:.1}%)", self.main_cost, pct));

        let pct = (self.opcode_cost as f64 / total as f64) * 100.0;
        parts.push(format!("opcode={} ({:.1}%)", self.opcode_cost, pct));

        let pct = (self.memory_cost as f64 / total as f64) * 100.0;
        parts.push(format!("memory={} ({:.1}%)", self.memory_cost, pct));

        let pct = (self.precompile_cost as f64 / total as f64) * 100.0;
        parts.push(format!("precompile={} ({:.1}%)", self.precompile_cost, pct));

        let pct = (self.tables_cost as f64 / total as f64) * 100.0;
        parts.push(format!("tables={} ({:.1}%)", self.tables_cost, pct));

        if self.other_cost > 0 {
            let pct = (self.other_cost as f64 / total as f64) * 100.0;
            parts.push(format!("other={} ({:.1}%)", self.other_cost, pct));
        }

        write!(f, "total={} [{}]", total, parts.join(", "))
    }
}

/// Struct to hold timing information for different phases of the Zisk executor.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZiskExecutorTime {
    /// Total executor duration of the entire execution process.
    pub total_duration: u64,
    /// Duration of the execution phase.
    pub execution_duration: u64,
    /// Duration of the counting and planning phase for main state machines.
    pub count_and_plan_duration: u64,
    /// Duration of the counting and planning phase for memory operations from ASM runner.
    pub count_and_plan_mo_duration: u64,
    /// Execution duration of the ASM runner.
    pub asm_execution_duration: Option<AsmExecutionInfo>,
}

/// Per-AIR planned instance count. Plain `(airgroup_id, air_id, count)` triple so it can
/// live in `common` (no executor dependency) and cross the wire; the human-readable AIR
/// name is derived from the ids by the consumer.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct AirInstanceCount {
    /// AIR group ID.
    pub airgroup_id: usize,
    /// AIR ID.
    pub air_id: usize,
    /// Number of instances.
    pub count: u64,
}

/// Struct to hold a summary of the Zisk executor's performance and cost metrics.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZiskExecutorSummary {
    /// Total number of steps executed by the Zisk executor.
    pub steps: u64,
    /// Timing information for different phases of the Zisk executor.
    pub executor_time: ZiskExecutorTime,
    /// Cost breakdown by different types of operations.
    pub cost_per_type: StatsCostPerType,
    /// Per-AIR instance plan. Empty unless produced by a planning executor run.
    pub plan: Vec<AirInstanceCount>,
}

impl ZiskExecutorSummary {
    /// Creates a new `ZiskExecutorSummary` with the given metrics.
    pub fn new(
        executed_steps: u64,
        execution_time: ZiskExecutorTime,
        cost_per_type: StatsCostPerType,
    ) -> Self {
        Self {
            steps: executed_steps,
            executor_time: execution_time,
            cost_per_type,
            ..Default::default()
        }
    }
}

/// Struct to hold detailed timing and chunk information for a specific AIR instance.
#[derive(Debug, Clone)]
pub struct Stats {
    /// AIR group ID.
    pub airgroup_id: usize,
    /// AIR ID.
    pub air_id: usize,
    /// Collect start time
    pub collect_start_time: Instant,
    /// Collect duration in microseconds
    pub collect_duration: u64,
    /// Witness start time
    pub witness_start_time: Instant,
    /// Witness duration in microseconds
    pub witness_duration: u128,
    /// Number of chunks
    pub num_chunks: usize,
}

impl Stats {
    /// Creates stats for an instance with no collection phase.
    ///
    /// Used for main instances and ROM instances with ASM emulator that skip collection.
    /// Sets `collect_duration` to 0 and `num_chunks` to 0.
    pub fn new_no_collection(airgroup_id: usize, air_id: usize) -> Self {
        Self {
            airgroup_id,
            air_id,
            collect_start_time: Instant::now(),
            collect_duration: 0,
            witness_start_time: Instant::now(),
            witness_duration: 0,
            num_chunks: 0,
        }
    }

    /// Creates stats for an instance with a pending collection phase.
    ///
    /// Used when collection is about to start. The `collect_duration` will be
    /// updated later via `set_collect_duration` when collection completes.
    pub fn new_pending_collection(airgroup_id: usize, air_id: usize, num_chunks: usize) -> Self {
        Self {
            airgroup_id,
            air_id,
            collect_start_time: Instant::now(),
            collect_duration: 0,
            witness_start_time: Instant::now(),
            witness_duration: 0,
            num_chunks,
        }
    }

    /// Creates stats for an instance with completed collection.
    ///
    /// Used when collection has finished and we know the actual timing.
    pub fn new_with_collection(
        airgroup_id: usize,
        air_id: usize,
        num_chunks: usize,
        collect_start_time: Instant,
        collect_duration: u64,
    ) -> Self {
        Self {
            airgroup_id,
            air_id,
            collect_start_time,
            collect_duration,
            witness_start_time: Instant::now(),
            witness_duration: 0,
            num_chunks,
        }
    }

    /// Creates stats for a main instance (no collection, witness already computed).
    ///
    /// Used when witness computation has finished and we know the timing.
    /// Main instances don't have a collection phase.
    pub fn new_main_completed(
        airgroup_id: usize,
        air_id: usize,
        witness_start_time: Instant,
    ) -> Self {
        Self {
            airgroup_id,
            air_id,
            collect_start_time: Instant::now(),
            collect_duration: 0,
            witness_start_time,
            witness_duration: witness_start_time.elapsed().as_millis(),
            num_chunks: 0,
        }
    }
}

/// Struct to hold timing information for assembly execution, including total time and effective MHz.
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsmExecutionInfo {
    /// Total time taken for the assembly execution in seconds.
    pub time: f32,
    /// Effective execution speed in MHz.
    pub mhz: f32,
}

impl fmt::Display for AsmExecutionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}s ({:.0} MHz)", self.time, self.mhz)
    }
}
