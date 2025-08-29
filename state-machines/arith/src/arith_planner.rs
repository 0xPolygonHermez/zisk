//! The `ArithPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use std::any::Any;

use crate::ArithCounterInputGen;
use std::collections::HashMap;
use zisk_common::CollectSkipper;
use zisk_common::{
    plan_with_frops, BusDeviceMetrics, CheckPoint, ChunkId, InstFropsCount, InstanceInfo,
    InstanceType, Metrics, Plan, Planner, TableInfo,
};

/// The `ArithPlanner` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct ArithPlanner {
    /// Arithmetic instances info to be planned.
    instances_info: Vec<InstanceInfo>,

    /// Arithmetic table instances info to be planned.
    tables_info: Vec<TableInfo>,
}

impl ArithPlanner {
    /// Creates a new `ArithPlanner`.
    ///
    /// # Returns
    /// A new `ArithPlanner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self { instances_info: Vec::new(), tables_info: Vec::new() }
    }

    /// Adds an arithmetic instance to the planner.
    ///
    /// # Arguments
    /// * `instance_info` - The `InstanceInfo` describing the arithmetic instance to be added.
    ///
    /// # Returns
    /// The updated `ArithPlanner` instance.
    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }
}

impl Planner for ArithPlanner {
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
        let mut count: Vec<Vec<InstFropsCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        counters.iter().for_each(|(chunk_id, counter)| {
            let reg_counter =
                Metrics::as_any(&**counter).downcast_ref::<ArithCounterInputGen>().unwrap();

            // Iterate over `instances_info` and add `InstCount` objects to the correct vector
            for (index, instance_info) in self.instances_info.iter().enumerate() {
                let inst_count = InstFropsCount::new(
                    *chunk_id,
                    reg_counter.inst_count(instance_info.op_type).unwrap(),
                    reg_counter.frops_count(instance_info.op_type).unwrap(),
                );

                // Add the `InstCount` to the corresponding inner vector
                count[index].push(inst_count);
            }
        });

        let mut plan_result = Vec::new();

        for (idx, instance) in self.instances_info.iter().enumerate() {
            let plan: Vec<_> = plan_with_frops(&count[idx], instance.num_ops as u64)
                .into_iter()
                .map(|(check_point, collect_info)| {
                    let converted: Box<dyn Any> = Box::new(collect_info);

                    // Downcast to access the data you need
                    let collect_info_ref = converted
                        .downcast_ref::<HashMap<ChunkId, (u64, bool, CollectSkipper)>>()
                        .expect("Failed to downcast collect_info to expected type");

                    let num_rows: u64 = collect_info_ref.values().map(|(v, _, _)| *v).sum();

                    Plan::new(
                        instance.airgroup_id,
                        instance.air_id,
                        Some(num_rows as usize),
                        None,
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                        4,
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
                    Some(table_instance.num_rows),
                    None,
                    InstanceType::Table,
                    CheckPoint::None,
                    None,
                    1,
                ));
            }
        }

        plan_result
    }
}
