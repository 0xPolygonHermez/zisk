//! The `Planner` module provides core structures and traits for organizing and managing
//! execution plans. It defines the `Plan` structure, `Planner` trait, and utility types
//! like `CheckPoint` and `CollectSkipper` for efficient planning and execution flows.

use std::any::Any;

use crate::{BusDeviceMetrics, ChunkId, InstanceType, SegmentId};

/// The `CollectSkipper` struct defines logic for skipping instructions during input collection.
///
/// This utility helps manage scenarios where a specific number of instructions need to be skipped
/// before processing resumes.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CollectSkipper {
    /// Number of instructions to be skipped.
    pub skip: u64,

    /// Number of already skipped instrucions.
    pub skipped: u64,

    /// Flag indicating whether skipping is active.
    pub skipping: bool,
}

impl CollectSkipper {
    /// Creates a new `CollectSkipper` instance.
    ///
    /// # Arguments
    /// * `skip` - The number of instructions to skip.
    ///
    /// # Returns
    /// A new `CollectSkipper` instance with initial settings.
    pub fn new(skip: u64) -> Self {
        CollectSkipper { skip, skipped: 0, skipping: skip > 0 }
    }

    /// Determines whether the current instruction should be skipped.
    ///
    /// # Returns
    /// `true` if the instruction should be skipped, `false` otherwise.
    #[inline(always)]
    pub fn should_skip(&mut self) -> bool {
        if !self.skipping {
            return false;
        }

        if self.skip == 0 || self.skipped >= self.skip {
            self.skipping = false;
            return false;
        }

        self.skipped += 1;
        true
    }

    #[inline(always)]
    pub fn should_skip_query(&mut self, apply: bool) -> bool {
        if !self.skipping {
            return false;
        }

        if self.skip == 0 || self.skipped >= self.skip {
            self.skipping = false;
            return false;
        }

        if apply {
            self.skipped += 1;
        }
        true
    }
}

/// The `CollectCounter` struct defines logic for a three-phase collection strategy.
///
/// Phase 1: Skip initial elements
/// Phase 2: Collect (don't skip) a specified number of elements  
/// Phase 3: Skip all remaining elements
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CollectCounter {
    /// Number of initial elements to skip
    pub initial_skip: u32,

    /// Number of elements already skipped in initial phase
    pub initial_skipped: u32,

    /// Number of elements to collect (not skip) after initial skip
    pub collect_count: u32,

    /// Number of elements already collected
    pub collected: u32,

    /// Flag indicating whether we're in initial skip phase
    pub initial_skipping: bool,

    /// Flag indicating whether we're in final skip-all phase
    pub final_skip_phase: bool,
}

impl CollectCounter {
    /// Creates a new `CollectCounter` instance.
    ///
    /// # Arguments
    /// * `initial_skip` - Number of elements to skip at the beginning
    /// * `collect_count` - Number of elements to collect after initial skip
    ///
    /// # Returns
    /// A new `CollectCounter` with the specified behavior
    pub fn new(initial_skip: u32, collect_count: u32) -> Self {
        CollectCounter {
            initial_skip,
            initial_skipped: 0,
            collect_count,
            collected: 0,
            initial_skipping: initial_skip > 0,
            final_skip_phase: false,
        }
    }

    /// Determines whether the current instruction should be skipped.
    ///
    /// Behavior:
    /// 1. Skip first `initial_skip` elements
    /// 2. Don't skip next `collect_count` elements  
    /// 3. Skip all remaining elements
    #[inline(always)]
    pub fn should_skip(&mut self) -> bool {
        // Phase 1: Initial skipping
        if self.initial_skipping {
            if self.initial_skip == 0 || self.initial_skipped >= self.initial_skip {
                self.initial_skipping = false;
            } else {
                self.initial_skipped += 1;
                return true;
            }
        }

        // Phase 2: Collecting (not skipping)
        if self.collected < self.collect_count {
            self.collected += 1;
            return false;
        }

        // Phase 3: Skip all remaining elements
        self.final_skip_phase = true;
        true
    }

    /// Reset to initial state with new parameters
    pub fn reset(&mut self, initial_skip: u32, collect_count: u32) {
        self.initial_skip = initial_skip;
        self.initial_skipped = 0;
        self.collect_count = collect_count;
        self.collected = 0;
        self.initial_skipping = initial_skip > 0;
        self.final_skip_phase = false;
    }

    /// Returns the current phase as a string
    pub fn get_phase(&self) -> &str {
        if self.initial_skipping {
            "initial_skip"
        } else if self.collected < self.collect_count {
            "collecting"
        } else {
            "final_skip"
        }
    }

    /// Returns whether we're currently in the collecting phase
    pub fn is_collecting(&self) -> bool {
        !self.initial_skipping && self.collected < self.collect_count
    }

    /// Returns whether we're in the final skip phase
    pub fn is_final_skip(&self) -> bool {
        self.final_skip_phase
    }

    /// Returns number of elements remaining to collect
    pub fn remaining_to_collect(&self) -> u32 {
        self.collect_count.saturating_sub(self.collected)
    }
    pub fn count(&self) -> u32 {
        self.collect_count
    }
    pub fn skip(&self) -> u32 {
        self.initial_skip
    }
}

/// Represents different types of checkpoints in a plan.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckPoint {
    /// No checkpoint.
    None,

    /// A single chunk checkpoint.
    Single(ChunkId),

    /// Multiple chunk checkpoints.
    Multiple(Vec<ChunkId>),
}

/// The `Plan` struct represents a single execution plan.
#[derive(Debug)]
pub struct Plan {
    /// The AIR group ID.
    pub airgroup_id: usize,

    /// The AIR ID.
    pub air_id: usize,

    /// The segment ID associated with this plan.
    pub segment_id: Option<SegmentId>,

    /// The type of instance associated with this plan.
    pub instance_type: InstanceType,

    /// The checkpoint type associated with this plan.
    pub check_point: CheckPoint,

    /// Additional metadata associated with the plan.
    pub meta: Option<Box<dyn Any>>,

    pub global_id: Option<usize>,
}

impl Plan {
    /// Creates a new `Plan` instance.
    ///
    /// # Arguments
    /// * `airgroup_id` - The AIR group ID.
    /// * `air_id` - The AIR ID.
    /// * `segment_id` - The segment ID (if any).
    /// * `instance_type` - The type of instance.
    /// * `check_point` - The checkpoint type.
    /// * `collect_info` - Optional input collection information.
    /// * `meta` - Optional additional metadata.
    ///
    /// # Returns
    /// A new `Plan` instance with the specified settings.
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        segment_id: Option<SegmentId>,
        instance_type: InstanceType,
        check_point: CheckPoint,
        meta: Option<Box<dyn Any>>,
    ) -> Self {
        Plan { airgroup_id, air_id, segment_id, instance_type, check_point, meta, global_id: None }
    }

    pub fn set_global_id(&mut self, global_id: usize) {
        self.global_id = Some(global_id);
    }
}

unsafe impl Send for Plan {}
unsafe impl Sync for Plan {}

/// The `Planner` trait defines the interface for creating execution plans.
///
/// Implementers of this trait must define how plans are generated from input metrics.
pub trait Planner {
    /// Generates a vector of `Plan` instances based on provided metrics.
    ///
    /// # Arguments
    /// * `counter` - A vector of tuples where:
    ///   - The first element is a `ChunkId` identifying the metric's source.
    ///   - The second element is a boxed implementation of `BusDeviceMetrics`.
    ///
    /// # Returns
    /// A vector of `Plan` instances.
    fn plan(&self, counter: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan>;
}
