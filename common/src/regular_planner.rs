//! The `RegularPlanner` module provides a planner for organizing and generating execution plans
//! for regular instances and table instances. It leverages operation counts and metadata
//! to construct detailed plans for execution.

use std::any::Any;

use crate::{
    BusDeviceMetrics, CheckPoint, ChunkId, InstCount, InstanceType, Metrics, Plan, Planner,
    RegularCounters,
};
use zisk_core::ZiskOperationType;

use super::plan;

/// Metadata about an instance to be planned.
///
/// This includes details such as the AIR group, AIR ID, and the number of rows required for the
/// instance.
#[derive(Debug)]
pub struct InstanceInfo {
    /// The AIR group ID.
    pub airgroup_id: usize,

    /// The AIR ID.
    pub air_id: usize,

    /// The number of operations required by the instance.
    pub num_ops: usize,

    /// The `ZiskOperationType` associated with the instance.
    pub op_type: ZiskOperationType,
}

impl InstanceInfo {
    /// Creates a new `InstanceInfo`.
    ///
    /// # Arguments
    /// * `air_id` - The AIR ID.
    /// * `airgroup_id` - The AIR group ID.
    /// * `num_ops` - The number of operations for this instance.
    /// * `op_type` - The operation type associated with the instance.
    ///
    /// # Returns
    /// A new `InstanceInfo` instance.
    pub fn new(
        airgroup_id: usize,
        air_id: usize,
        num_ops: usize,
        op_type: ZiskOperationType,
    ) -> Self {
        InstanceInfo { air_id, airgroup_id, num_ops, op_type }
    }
}

/// Metadata about a table to be planned.
///
/// This includes details such as the AIR group and AIR ID.
pub struct TableInfo {
    /// The AIR group ID.
    pub airgroup_id: usize,

    /// The AIR ID.
    pub air_id: usize,
}

impl TableInfo {
    /// Creates a new `TableInfo`.
    ///
    /// # Arguments
    /// * `air_id` - The AIR ID.
    /// * `airgroup_id` - The AIR group ID.
    ///
    /// # Returns
    /// A new `TableInfo` instance.
    pub fn new(airgroup_id: usize, air_id: usize) -> Self {
        TableInfo { air_id, airgroup_id }
    }
}
/// The `RegularPlanner` struct organizes execution plans for regular and table instances.
///
/// It supports adding metadata about instances and tables, and generates plans by leveraging
/// counters.
#[derive(Default)]
pub struct RegularPlanner {
    /// Metadata about instances to be planned.
    instances_info: Vec<InstanceInfo>,

    /// Metadata about table instances to be planned.
    tables_info: Vec<TableInfo>,
}

impl RegularPlanner {
    /// Creates a new `RegularPlanner`.
    ///
    /// # Returns
    /// A new `RegularPlanner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self { instances_info: Vec::new(), tables_info: Vec::new() }
    }

    /// Adds an instance to the planner.P
    ///
    /// # Arguments
    /// * `instance_info` - The `InstanceInfo` describing the instance to be added.
    ///
    /// # Returns
    /// The updated `RegularPlanner` instance.
    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }

    /// Adds a table instance to the planner.
    ///
    /// # Arguments
    /// * `table_info` - The `TableInfo` describing the table to be added.
    ///
    /// # Returns
    /// The updated `RegularPlanner` instance.
    pub fn add_table_instance(mut self, table_info: TableInfo) -> Self {
        self.tables_info.push(table_info);
        self
    }
}

impl Planner for RegularPlanner {
    /// Generates execution plans for regular and table instances.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances and
    /// tables.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Prepare counts
        let mut count: Vec<Vec<InstCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        counters.iter().for_each(|(chunk_id, counter)| {
            let reg_counter =
                Metrics::as_any(&**counter).downcast_ref::<RegularCounters>().unwrap();

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
