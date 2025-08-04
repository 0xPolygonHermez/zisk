//! The `RomPlanner` module defines a planner for organizing execution plans for ROM-related
//! operations. It aggregates ROM metrics and generates a plan for the execution flow.

use zisk_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Plan, Planner};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

/// The `RomPlanner` struct creates an execution plan from aggregated ROM metrics.
///
/// It processes metrics collected by `RomCounter` instances, combines them,
/// and generates a single `Plan` for execution.
pub struct RomPlanner;

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

        let vec_chunk_ids = metrics.iter().map(|(chunk_id, _)| *chunk_id).collect::<Vec<_>>();

        vec![Plan::new(
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            None,
            InstanceType::Instance,
            CheckPoint::Multiple(vec_chunk_ids),
            None,
        )]
    }
}
