//! The `ArithEqPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use std::any::Any;

use crate::ArithEqCounterInputGen;

use zisk_common::{
    plan, BusDeviceMetrics, CheckPoint, ChunkId, InstCount, InstanceInfo, InstanceType, Metrics,
    Plan, Planner, TableInfo,
};

/// The `ArithEqPlanner` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct ArithEqPlanner {
    /// Arithmetic instances info to be planned.
    instances_info: Vec<InstanceInfo>,

    /// Arithmetic table instances info to be planned.
    tables_info: Vec<TableInfo>,
}

impl ArithEqPlanner {
    /// Creates a new `ArithEqPlanner`.
    ///
    /// # Returns
    /// A new `Arith256Planner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self { instances_info: Vec::new(), tables_info: Vec::new() }
    }

    /// Adds an arithmetic instance to the planner.
    ///
    /// # Arguments
    /// * `instance_info` - The `InstanceInfo` describing the arithmetic instance to be added.
    ///
    /// # Returns
    /// The updated `ArithEqPlanner` instance.
    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }

    /// Adds an arithmetic table instance to the planner.
    ///
    /// # Arguments
    /// * `table_info` - The `TableInfo` describing the arithmetic table instance to be added.
    ///
    /// # Returns
    /// The updated `ArithEqPlanner` instance.
    pub fn add_table_instance(mut self, table_info: TableInfo) -> Self {
        self.tables_info.push(table_info);
        self
    }
}

impl Planner for ArithEqPlanner {
    /// Generates execution plans for arithmetic instances and tables.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and `ArithCounter`
    ///   metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances and
    /// tables.
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to an `ArithCounter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Prepare counts
        let mut count: Vec<Vec<InstCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        counters.iter().for_each(|(chunk_id, counter)| {
            let reg_counter =
                Metrics::as_any(&**counter).downcast_ref::<ArithEqCounterInputGen>().unwrap();

            // Iterate over `instances_info` and add `InstCount` objects to the correct vector
            for (index, instance_info) in self.instances_info.iter().enumerate() {
                let inst_count = InstCount::new(
                    *chunk_id,
                    reg_counter.inst_count(instance_info.op_type).unwrap(),
                );

                // Add the `InstCount` to the corresponding inner vector
                count[index].push(inst_count);
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

        if !plan_result.is_empty() {
            for table_instance in self.tables_info.iter() {
                plan_result.push(Plan::new(
                    table_instance.airgroup_id,
                    table_instance.air_id,
                    None,
                    InstanceType::Table,
                    CheckPoint::None,
                    None,
                ));
            }
        }
        plan_result
    }
}
