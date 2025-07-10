//! The `Planner` module provides core structures and traits for organizing and managing
//! execution plans. It defines the `Plan` structure, `Planner` trait, and utility types
//! like `CheckPoint` and `CollectSkipper` for efficient planning and execution flows.

use std::any::Any;

use proofman_common::PreCalculate;

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

    pub pre_calculate: PreCalculate,
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
        pre_calculate: PreCalculate,
        meta: Option<Box<dyn Any>>,
    ) -> Self {
        Plan {
            airgroup_id,
            air_id,
            segment_id,
            instance_type,
            check_point,
            meta,
            pre_calculate,
            global_id: None,
        }
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
