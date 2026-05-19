//! The `RomPlanner` module defines a planner for organizing execution plans for ROM-related
//! operations. It aggregates ROM metrics and generates a plan for the execution flow.

use zisk_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Plan, Planner};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

/// The `RomPlanner` struct creates an execution plan from aggregated ROM metrics.
///
/// It processes metrics collected by `RomCounter` instances, combines them,
/// and generates a single `Plan` for execution.
pub(crate) struct RomPlanner;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use zisk_common::{BusDevice, Metrics};

    /// Minimal `BusDeviceMetrics` impl — `RomPlanner::plan` only reads the `ChunkId`s,
    /// so the payload type can be a zero-sized stand-in.
    struct DummyMetrics;
    impl Metrics for DummyMetrics {
        fn measure(&mut self, _: &[u64]) {}
        fn as_any(&self) -> &dyn Any {
            self
        }
    }
    impl BusDevice<u64> for DummyMetrics {
        fn as_any(self: Box<Self>) -> Box<dyn Any> {
            self
        }
    }

    #[test]
    #[should_panic(expected = "No metrics found")]
    fn plan_panics_on_empty_metrics() {
        let _ = RomPlanner.plan(Vec::new());
    }

    #[test]
    fn plan_aggregates_chunk_ids_into_single_multiple_checkpoint() {
        let metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)> = vec![
            (ChunkId(0), Box::new(DummyMetrics)),
            (ChunkId(2), Box::new(DummyMetrics)),
            (ChunkId(7), Box::new(DummyMetrics)),
        ];

        let plans = RomPlanner.plan(metrics);

        assert_eq!(plans.len(), 1, "always exactly one Plan");
        let p = &plans[0];
        assert_eq!(p.airgroup_id, ZISK_AIRGROUP_ID);
        assert_eq!(p.air_id, ROM_AIR_IDS[0]);
        assert_eq!(p.instance_type, InstanceType::Instance);
        match &p.check_point {
            CheckPoint::Multiple(ids) => {
                assert_eq!(ids, &vec![ChunkId(0), ChunkId(2), ChunkId(7)]);
            }
            other => panic!("expected CheckPoint::Multiple, got {other:?}"),
        }
    }
}
