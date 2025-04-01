//! The `RomPlanner` module defines a planner for organizing execution plans for ROM-related
//! operations. It aggregates ROM metrics and generates a plan for the execution flow.

use std::sync::{atomic::AtomicU32, Arc};

use sm_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Metrics, Plan, Planner};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::RomCounter;

/// The `RomPlanner` struct creates an execution plan from aggregated ROM metrics.
///
/// It processes metrics collected by `RomCounter` instances, combines them,
/// and generates a single `Plan` for execution.
pub struct RomPlanner {
    /// Shared biod instruction counter for monitoring ROM operations.
    bios_inst_count: Arc<Vec<AtomicU32>>,

    /// Shared program instruction counter for monitoring ROM operations.
    prog_inst_count: Arc<Vec<AtomicU32>>,
}

impl RomPlanner {
    /// Creates a new instance of `RomPlanner`.
    ///
    /// # Returns
    /// A new `RomPlanner` instance.
    pub fn new(bios_inst_count: Arc<Vec<AtomicU32>>, prog_inst_count: Arc<Vec<AtomicU32>>) -> Self {
        Self { bios_inst_count, prog_inst_count }
    }
}

impl Planner for RomPlanner {
    /// Creates an execution plan based on ROM metrics.
    ///
    /// This method collects metrics from `RomCounter` instances, aggregates them,
    /// and constructs a single plan that includes the combined metrics.
    ///
    /// # Arguments
    /// * `metrics` - A vector of tuples where:
    ///   - The first element is a `ChunkId` that identifies the source of the metric.
    ///   - The second element is a boxed implementation of `BusDeviceMetrics`, which must be a
    ///     `RomCounter`.
    ///
    /// # Returns
    /// A vector containing one `Plan` instance that defines the execution flow for ROM operations.
    ///
    /// # Panics
    /// This method panics if the `metrics` vector is empty.
    fn plan(&self, metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        if metrics.is_empty() {
            panic!("RomPlanner::plan() No metrics found");
        }

        let mut total = RomCounter::new(self.bios_inst_count.clone(), self.prog_inst_count.clone());

        for (_, metric) in metrics {
            let metric = Metrics::as_any(&*metric).downcast_ref::<RomCounter>().unwrap();
            total += metric;
        }

        vec![Plan::new(
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            None,
            InstanceType::Instance,
            CheckPoint::None,
            Some(Box::new(total)),
        )]
    }
}
