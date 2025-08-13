//! The `FrequentOpsPlanner` module defines a planner for organizing execution plans for frequent operations.
//! It aggregates frequent operations metrics and generates a plan for the execution flow.

use proofman_common::PreCalculate;
use zisk_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Plan, Planner};
use zisk_pil::{FREQUENT_OPS_AIR_IDS, ZISK_AIRGROUP_ID};

/// The `FrequentOpsPlanner` struct creates an execution plan from aggregated frequent operations metrics.
///
/// It processes metrics collected by `FrequentOpsCounter` instances, combines them,
/// and generates a single `Plan` for execution.
pub struct FrequentOpsPlanner;

impl Planner for FrequentOpsPlanner {
    /// Creates an execution plan based on frequent operations metrics.
    ///
    /// This method collects metrics from `FrequentOpsCounter` instances, aggregates them,
    /// and constructs a single plan that includes the combined metrics.
    ///
    /// # Arguments
    /// * `metrics` - A vector of tuples where:
    ///   - The first element is a `ChunkId` that identifies the source of the metric.
    ///   - The second element is a boxed implementation of `BusDeviceMetrics`, which must be a
    ///     `FrequentOpsCounter`.
    ///
    /// # Returns
    /// A vector containing one `Plan` instance that defines the execution flow for frequent operations.
    ///
    /// # Panics
    /// This method panics if the `metrics` vector is empty.
    fn plan(&self, metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let all_chunks = metrics.iter().map(|(chunk_id, _)| *chunk_id).collect::<Vec<_>>();
        vec![Plan::new(
            ZISK_AIRGROUP_ID,
            FREQUENT_OPS_AIR_IDS[0],
            None,
            InstanceType::Instance,
            CheckPoint::Multiple(all_chunks),
            PreCalculate::None,
            None,
        )]
    }
}
