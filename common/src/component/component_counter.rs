//! The `Metrics` and `Counter` modules define traits and structures for tracking and aggregating
//! performance metrics. These modules provide flexible mechanisms to count and analyze
//! bus operations and instruction execution in a fine-grained manner.

use std::{
    any::Any,
    fmt::Debug,
    ops::{Add, AddAssign},
    sync::{atomic::AtomicU32, Arc},
};

use zisk_core::{ROM_ADDR, ROM_ENTRY};

/// The `Metrics` trait provides an interface for tracking and managing metrics in a
/// flexible and extensible manner.
///
/// Implementers of this trait can measure data associated with bus IDs, merge metrics
/// from other sources, and retrieve associated bus IDs.
pub trait Metrics: Send + Sync {
    /// Measures and processes data associated with a specific bus ID.
    ///
    /// # Arguments
    /// * `data` - The payload data associated with the bus ID.
    ///
    /// # Returns
    /// A vector of tuples containing:
    /// - The bus ID for derived data.
    /// - The derived data payload.
    fn measure(&mut self, data: &[u64]);

    /// Provides a dynamic reference for type casting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn Any`.
    fn as_any(&self) -> &dyn Any;
}

/// The `Counter` struct represents a simple counter for tracking the number of instructions
/// executed.
#[derive(Default, Debug, Clone)]
pub struct Counter {
    /// The total number of counted instructions.
    pub inst_count: u64,
}

impl Counter {
    /// Updates the counter by incrementing it with a given value.
    ///
    /// # Arguments
    /// * `num` - The number of instructions to add to the counter.
    #[inline(always)]
    pub fn update(&mut self, num: u64) {
        self.inst_count += num;
    }
}

impl Add for &Counter {
    type Output = Counter;

    /// Adds two counters and returns a new `Counter` instance with the combined count.
    ///
    /// # Arguments
    /// * `self` - The first counter.
    /// * `other` - The second counter to be added.
    ///
    /// # Returns
    /// A new `Counter` instance with the combined instruction count.
    fn add(self, other: Self) -> Counter {
        Counter { inst_count: self.inst_count + other.inst_count }
    }
}

impl AddAssign<&Counter> for Counter {
    /// Adds the count of another counter to the current counter.
    ///
    /// # Arguments
    /// * `other` - The counter to add.
    fn add_assign(&mut self, other: &Counter) {
        self.inst_count += other.inst_count;
    }
}

/// The `CounterStats` struct provides detailed metrics for instruction execution,
/// tracking counts by program counter (PC) and execution steps.
#[derive(Debug)]
pub struct CounterStats {
    /// Shared biod instruction counter for monitoring ROM operations.
    pub bios_inst_count: Arc<Vec<AtomicU32>>,

    /// Shared program instruction counter for monitoring ROM operations.
    pub prog_inst_count: Arc<Vec<AtomicU32>>,

    /// The PC of the last executed instruction.
    pub end_pc: u64,

    /// The total number of executed instructions (steps).
    pub steps: u64,
}

impl CounterStats {
    pub fn new(entry_inst_count: Arc<Vec<AtomicU32>>, inst_count: Arc<Vec<AtomicU32>>) -> Self {
        CounterStats {
            bios_inst_count: entry_inst_count,
            prog_inst_count: inst_count,
            end_pc: 0,
            steps: 0,
        }
    }

    /// Updates the counter statistics with information about the current instruction execution.
    ///
    /// # Arguments
    /// * `pc` - The program counter (PC) of the executed instruction.
    /// * `step` - The current execution step.
    /// * `num` - The number of instructions executed at the given PC.
    /// * `end` - A flag indicating if this is the final instruction in the execution.
    #[inline(always)]
    pub fn update(&mut self, pc: u64, step: u64, num: u32, end: bool) {
        if pc < ROM_ADDR {
            let addr = ((pc - ROM_ENTRY) as usize) >> 2;
            self.bios_inst_count[addr].fetch_add(num, std::sync::atomic::Ordering::Relaxed);
        } else {
            let addr = (pc - ROM_ADDR) as usize;
            self.prog_inst_count[addr].fetch_add(num, std::sync::atomic::Ordering::Relaxed);
        }

        if end {
            self.end_pc = pc;
            self.steps = step + 1;
        }
    }
}

impl AddAssign<&CounterStats> for CounterStats {
    /// Merges the metrics of another `CounterStats` instance into the current instance.
    ///
    /// # Arguments
    /// * `other` - The `CounterStats` instance to merge.
    fn add_assign(&mut self, other: &CounterStats) {
        if other.end_pc != 0 {
            self.end_pc = other.end_pc;
        }

        if other.steps != 0 {
            self.steps = other.steps;
        }
    }
}
