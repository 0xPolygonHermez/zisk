//! The `DmaPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use crate::{DmaStrategy, DMA_WPP_CLASSES, DMA_WPP_CLASS_ROWS};

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
            // Rows one instance of this air holds; the planner budgets in rows, so the occupancy
            // is measured in them too.
            let capacity = DmaStrategy::<F>::rows_by_air_id(air_id);
            for (segment_id, (check_point, collect_info)) in segments.into_iter().enumerate() {
                // Rows this segment was budgeted, over every chunk and operation.
                let used: u64 = collect_info
                    .chunks
                    .values()
                    .map(|(_, counters)| counters.total_collect_count())
                    .sum();
                let plan = Plan::new(
                    ZISK_AIRGROUP_ID,
                    air_id,
                    Some(SegmentId(segment_id)),
                    InstanceType::Instance,
                    check_point.clone(),
                    Some(Box::new(collect_info)),
                );
                plans.push(match capacity {
                    Some(capacity) => plan.with_occupancy(used, capacity as u64),
                    None => plan,
                });
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
        let loop_capacity = DmaStrategy::<F>::rows_by_air_id(DmaLoopTrace::<F>::AIR_ID)
            .expect("DmaLoop is planned by the DMA strategy") as u64;
        for (segment_id, (check_point, collect_info)) in
            std::mem::take(&mut dma_strategy.dma_loop_plan).into_iter().enumerate()
        {
            let used: u64 = collect_info
                .chunks
                .values()
                .map(|(_, counters)| counters.total_collect_count())
                .sum();
            plans.push(
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    DmaLoopTrace::<F>::AIR_ID,
                    Some(SegmentId(segment_id)),
                    InstanceType::Instance,
                    check_point,
                    Some(Box::new(collect_info)),
                )
                .with_occupancy(used, loop_capacity),
            );
        }
        // One instance of the fused air is one of each of its blocks, so its occupancy is the rows
        // of the two together against the rows of the two.
        let compact_capacity = DmaStrategy::<F>::rows_by_air_id(CompactDmaTrace::<F>::AIR_ID)
            .expect("CompactDma is planned by the DMA strategy")
            as u64;
        for (segment_id, (check_point, collect_info)) in
            std::mem::take(&mut dma_strategy.compact_dma_plan).into_iter().enumerate()
        {
            let wpp_used: u64 = collect_info
                .wpp
                .chunks
                .values()
                .map(|(_, counters)| {
                    (0..DMA_WPP_CLASSES)
                        .map(|class| {
                            counters.classes[class].collect_count as u64
                                * DMA_WPP_CLASS_ROWS[class] as u64
                        })
                        .sum::<u64>()
                })
                .sum();
            let loop_used: u64 = collect_info
                .lp
                .chunks
                .values()
                .map(|(_, counters)| counters.total_collect_count())
                .sum();
            plans.push(
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    CompactDmaTrace::<F>::AIR_ID,
                    Some(SegmentId(segment_id)),
                    InstanceType::Instance,
                    check_point,
                    Some(Box::new(collect_info)),
                )
                .with_occupancy(wpp_used + loop_used, compact_capacity),
            );
        }
        plans
    }
}
