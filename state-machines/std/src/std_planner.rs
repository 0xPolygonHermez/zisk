//! The `StdPlanner` module defines a planner for generating execution plans based on
//! the PIL2 standard library. It organizes and creates plans for range table-based operations.

use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{BusDeviceMetrics, CheckPoint, ChunkId, InstanceType, Plan, Planner};

/// The `StdPlanner` struct generates execution plans using the PIL2 standard library.
///
/// This planner is designed to create execution plans for range tables by leveraging
/// metadata provided by the standard library.
pub struct StdPlanner<F: PrimeField> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,
}

impl<F: PrimeField> StdPlanner<F> {
    /// Creates a new instance of `StdPlanner`.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// A new `StdPlanner` instance.
    pub fn new(std: Arc<Std<F>>) -> Self {
        Self { std }
    }
}

impl<F: PrimeField> Planner for StdPlanner<F> {
    /// Generates execution plans using metadata from the PIL2 standard library.
    ///
    /// This method retrieves information about range tables from the standard library
    /// and constructs a plan for each range table.
    ///
    /// # Arguments
    /// * `_` - A vector of metrics, which is unused in this implementation.
    ///
    /// # Returns
    /// A vector of `Plan` instances, each representing a range table-based execution plan.
    fn plan(&self, _: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        self.std
            .get_ranges()
            .into_iter()
            .map(|(airgroup_id, air_id, rc_type)| {
                Plan::new(
                    airgroup_id,
                    air_id,
                    None,
                    InstanceType::Table,
                    CheckPoint::None,
                    Some(Box::new(rc_type)),
                )
            })
            .collect()
    }
}
