//! The `Add256Planner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use std::any::Any;

use crate::Add256CounterInputGen;

use zisk_common::{
    plan, BusDeviceMetrics, ChunkId, InstCount, InstanceInfo, InstanceType, Metrics, Plan, Planner,
};

/// The `Add256Planner` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct Add256Planner {
    /// Add256 instances info to be planned.
    instances_info: Vec<InstanceInfo>,
}

impl Add256Planner {
    /// Creates a new `Add256Planner`.
    ///
    /// # Returns
    /// A new `Add256Planner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self { instances_info: Vec::new() }
    }

    /// Adds an Add256 instance to the planner.
    ///
    /// # Arguments
    /// * `instance_info` - The `InstanceInfo` describing the add256 instance to be added.
    ///
    /// # Returns
    /// The updated `Add256Planner` instance.
    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }
}

impl Planner for Add256Planner {
    /// Generates execution plans for Add256 instances.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and `Add256Counter`
    ///   metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to an `Add256Counter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Prepare counts
        let mut count: Vec<Vec<InstCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        counters.iter().for_each(|(chunk_id, counter)| {
            let reg_counter =
                Metrics::as_any(&**counter).downcast_ref::<Add256CounterInputGen>().unwrap();

            // Iterate over `instances_info` and add `InstCount` objects to the correct vector
            for (index, instance_info) in self.instances_info.iter().enumerate() {
                if let Some(inst_count) = reg_counter.inst_count(instance_info.op_type) {
                    let inst_count = InstCount::new(*chunk_id, inst_count);
                    // Add the `InstCount` to the corresponding inner vector
                    count[index].push(inst_count);
                }
            }
        });

        let mut plan_result = Vec::new();

        for (idx, instance) in self.instances_info.iter().enumerate() {
            let plan: Vec<_> = plan(&count[idx], instance.num_ops as u64)
                .into_iter()
                .map(|(check_point, collect_info)| {
                    let converted: Box<dyn Any> = Box::new(collect_info);
                    Plan::new(
                        instance.airgroup_id,
                        instance.air_id,
                        None,
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                    )
                })
                .collect();

            plan_result.extend(plan);
        }

        plan_result
    }
}
