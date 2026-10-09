//! The `DmaPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use crate::DmaStrategy;

use proofman_fields::PrimeField64;
use zisk_common::{BusDeviceMetrics, ChunkId, InstanceType, Plan, Planner, SegmentId};
use zisk_pil::{CompactDmaTrace, DmaLoopTrace, DmaWithPrePostTrace, ZISK_AIRGROUP_ID};

/// The `DmaPlanner` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct DmaPlanner<F> {
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> DmaPlanner<F> {
    /// Creates a new `DmaPlanner`.
    ///
    /// # Returns
    /// A new `DmaPlanner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: PrimeField64> Planner for DmaPlanner<F> {
    /// Generates execution plans for Dma instances.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and `DmaCounter`
    ///   metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to an `DmaCounter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Calculate total counters by summing all DmaCounterInputGen instances
        let mut dma_strategy = DmaStrategy::<F>::default();
        let _plans = dma_strategy.calculate(counters);
        let mut plans: Vec<Plan> = Vec::new();
        for (air_id, segments) in _plans.into_iter() {
            for (segment_id, (check_point, collect_info)) in segments.into_iter().enumerate() {
                plans.push(Plan::new(
                    ZISK_AIRGROUP_ID,
                    air_id,
                    Some(SegmentId(segment_id)),
                    InstanceType::Instance,
                    check_point.clone(),
                    Some(Box::new(collect_info)),
                ));
            }
        }
        // The airs with a checkpoint type of their own come back apart from the rest; each one is
        // empty when the strategy gave that air nothing.
        for (segment_id, (check_point, collect_info)) in
            std::mem::take(&mut dma_strategy.dma_with_pre_post_plan).into_iter().enumerate()
        {
            plans.push(Plan::new(
                ZISK_AIRGROUP_ID,
                DmaWithPrePostTrace::<F>::AIR_ID,
                Some(SegmentId(segment_id)),
                InstanceType::Instance,
                check_point,
                Some(Box::new(collect_info)),
            ));
        }
        for (segment_id, (check_point, collect_info)) in
            std::mem::take(&mut dma_strategy.dma_loop_plan).into_iter().enumerate()
        {
            plans.push(Plan::new(
                ZISK_AIRGROUP_ID,
                DmaLoopTrace::<F>::AIR_ID,
                Some(SegmentId(segment_id)),
                InstanceType::Instance,
                check_point,
                Some(Box::new(collect_info)),
            ));
        }
        for (segment_id, (check_point, collect_info)) in
            std::mem::take(&mut dma_strategy.compact_dma_plan).into_iter().enumerate()
        {
            plans.push(Plan::new(
                ZISK_AIRGROUP_ID,
                CompactDmaTrace::<F>::AIR_ID,
                Some(SegmentId(segment_id)),
                InstanceType::Instance,
                check_point,
                Some(Box::new(collect_info)),
            ));
        }
        plans
    }
}
