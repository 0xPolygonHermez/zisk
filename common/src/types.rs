use std::fmt;
use std::time::Instant;

/// Type representing a chunk identifier.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId(pub usize);

impl ChunkId {
    pub const fn new(id: usize) -> Self {
        ChunkId(id)
    }

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
    pub const fn new(id: usize) -> Self {
        SegmentId(id)
    }

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

#[derive(Debug, Default, Clone)]
pub struct ZiskExecutionResult {
    pub steps: u64,
}

impl ZiskExecutionResult {
    pub fn new(executed_steps: u64) -> Self {
        Self { steps: executed_steps }
    }
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub airgroup_id: usize,
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

pub trait ElfBinaryLike {
    fn elf(&self) -> &[u8];
    fn name(&self) -> &str;
    fn with_hints(&self) -> bool;
}

pub struct ElfBinaryOwned {
    pub elf: Vec<u8>,
    pub name: String,
    pub with_hints: bool,
}

impl ElfBinaryOwned {
    pub const fn new(elf: Vec<u8>, name: String, with_hints: bool) -> Self {
        Self { elf, name, with_hints }
    }
}

impl ElfBinaryLike for ElfBinaryOwned {
    fn elf(&self) -> &[u8] {
        &self.elf
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn with_hints(&self) -> bool {
        self.with_hints
    }
}

pub struct ElfBinary {
    pub elf: &'static [u8],
    pub name: &'static str,
    pub with_hints: bool,
}

impl ElfBinaryLike for ElfBinary {
    fn elf(&self) -> &[u8] {
        self.elf
    }
    fn name(&self) -> &str {
        self.name
    }
    fn with_hints(&self) -> bool {
        self.with_hints
    }
}
