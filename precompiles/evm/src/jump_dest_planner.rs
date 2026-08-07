//! Planner for the `jump_dest` precompile.
//!
//! There is a single AIR to fill, so this needs none of the strategy machinery
//! the DMA planner carries — that one exists to route four operations across
//! four AIRs with different enables. Here the whole job is: take the rows each
//! chunk contributed, cut them into instances of the AIR's capacity, and emit
//! one plan per segment. The generic [`plan`] helper does the cutting, and the
//! `CollectSkipper` it returns is what lets a collector join a segment that
//! starts in the middle of a chunk.
//!
//! The unit is **rows, not operations**: a `jump_dest` spans
//! `ceil(count/64) * ROWS_X_BLOCK` rows, so counting operations would say
//! nothing about how many fit in an instance.

use std::collections::HashMap;

use fields::PrimeField64;
use zisk_common::{
    plan, BusDeviceMetrics, ChunkId, CollectSkipper, InstCount, InstanceType, Metrics, Plan,
    Planner, SegmentId,
};
use zisk_pil::{JumpDestTrace, JUMP_DEST_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{JumpDestCheckPoint, JumpDestCounterInputGen};

/// Cuts the row stream of the `jump_dest` operations into AIR-sized segments.
#[derive(Default)]
pub struct JumpDestPlanner<F> {
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> JumpDestPlanner<F> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: PrimeField64> Planner for JumpDestPlanner<F> {
    /// # Panics
    /// If a counter is not a `JumpDestCounterInputGen`, which would mean the bus
    /// routed another precompile's metrics here.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let counts: Vec<InstCount> = counters
            .iter()
            .map(|(chunk_id, counter)| {
                let rows = Metrics::as_any(&**counter)
                    .downcast_ref::<JumpDestCounterInputGen>()
                    .expect("JumpDestPlanner got a counter that is not a JumpDest one")
                    .rows;
                InstCount::new(*chunk_id, rows as u64)
            })
            .collect();

        let segments = plan(&counts, JumpDestTrace::<usize>::NUM_ROWS as u64);
        let last = segments.len().saturating_sub(1);

        segments
            .into_iter()
            .enumerate()
            .map(|(segment, (check_point, collect_info))| {
                let chunks: HashMap<ChunkId, (u64, CollectSkipper)> = collect_info;
                let meta = JumpDestCheckPoint {
                    last_chunk: chunks.keys().max().copied(),
                    is_last_segment: segment == last,
                    chunks,
                };
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    JUMP_DEST_AIR_IDS[0],
                    Some(SegmentId(segment)),
                    InstanceType::Instance,
                    check_point,
                    Some(Box::new(meta)),
                )
            })
            .collect()
    }
}
