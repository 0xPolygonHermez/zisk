//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! arithmetic operations.
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging arithmetic operation counts and metadata to construct detailed plans.

use std::{any::Any, mem};

use crate::BinaryCounterInputGen;
use sm_common::{
    BusDeviceMetrics, CheckPoint, ChunkId, InstanceInfo, InstanceType, Metrics, Plan, Planner,
    TableInfo,
};
use zisk_pil::{BinaryAddTrace, BinaryExtensionTrace, BinaryTrace};

/// The `BinaryPlanner` struct organizes execution plans for arithmetic instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct BinaryPlanner {
    /// Binary instances info to be planned.
    instances_info: Vec<InstanceInfo>,

    /// Binary table instances info to be planned.
    tables_info: Vec<TableInfo>,
}

impl BinaryPlanner {
    /// Creates a new `BinaryPlanner`.
    ///
    /// # Returns
    /// A new `BinaryPlanner` instance with no preconfigured instances or tables.
    pub fn new() -> Self {
        Self { instances_info: Vec::new(), tables_info: Vec::new() }
    }

    /// Adds an arithmetic instance to the planner.
    ///
    /// # Arguments
    /// * `instance_info` - The `InstanceInfo` describing the arithmetic instance to be added.
    ///
    /// # Returns
    /// The updated `BinaryPlanner` instance.
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
    /// The updated `BinaryPlanner` instance.
    pub fn add_table_instance(mut self, table_info: TableInfo) -> Self {
        self.tables_info.push(table_info);
        self
    }

    fn size_basic_of(rows: usize) -> usize {
        (rows / BinaryTrace::<usize>::NUM_ROWS) * BinaryTrace::<usize>::ROW_SIZE
    }

    fn size_basic_add_of(rows: usize) -> usize {
        (rows / BinaryAddTrace::<usize>::NUM_ROWS) * BinaryAddTrace::<usize>::ROW_SIZE
    }

    fn plan_for_extensions(&self, counters: Vec<(ChunkId, usize)>) -> Vec<Plan> {
        self.plan_for(
            BinaryExtensionTrace::<usize>::AIRGROUP_ID,
            BinaryExtensionTrace::<usize>::AIR_ID,
            BinaryExtensionTrace::<usize>::NUM_ROWS,
            counters,
            |skip| Box::new(skip),
        )
    }
    fn plan_for_basics(&self, counters: Vec<(ChunkId, usize)>, with_adds: bool) -> Vec<Plan> {
        self.plan_for(
            BinaryTrace::<usize>::AIRGROUP_ID,
            BinaryTrace::<usize>::AIR_ID,
            BinaryTrace::<usize>::NUM_ROWS,
            counters,
            |skip| Box::new((skip, with_adds)),
        )
    }
    fn plan_for_adds(&self, counters: Vec<(ChunkId, usize)>) -> Vec<Plan> {
        self.plan_for(
            BinaryAddTrace::<usize>::AIRGROUP_ID,
            BinaryAddTrace::<usize>::AIR_ID,
            BinaryAddTrace::<usize>::NUM_ROWS,
            counters,
            |skip| Box::new(skip),
        )
    }
    fn plan_for<F>(
        &self,
        airgroup_id: usize,
        air_id: usize,
        num_rows: usize,
        counters: Vec<(ChunkId, usize)>,
        func: F,
    ) -> Vec<Plan>
    where
        F: Fn(usize) -> Box<dyn Any>,
    {
        let mut plans: Vec<Plan> = Vec::new();
        let mut chunk_ids: Vec<ChunkId> = Vec::new();
        let mut available = num_rows;

        let last_index = counters.len() - 1;
        for (index, (chunk_id, counter)) in counters.iter().enumerate() {
            let mut pending = *counter;
            let flush_all = last_index == index;
            if pending > 0 {
                chunk_ids.push(*chunk_id);
            }
            let mut skip = 0;

            // exit from loop when no pending elements and no pending flush
            while pending > 0 || (flush_all && available < num_rows) {
                if available <= pending || flush_all {
                    plans.push(Plan::new(
                        airgroup_id,
                        air_id,
                        None,
                        InstanceType::Instance,
                        CheckPoint::Multiple(mem::take(&mut chunk_ids)),
                        Some(func(skip)),
                    ));
                    if available < pending {
                        skip += available;
                        pending -= available;
                        chunk_ids.push(*chunk_id);
                    } else {
                        pending = 0;
                    }
                    available = num_rows;
                } else {
                    available -= pending;
                    pending = 0;
                }
            }
        }
        plans
    }
}

impl Planner for BinaryPlanner {
    /// Generates execution plans for arithmetic instances and tables.
    ///
    /// # Arguments
    /// * `counters` - A vector of counters, each associated with a `ChunkId` and `BinaryCounter`
    ///   metrics data.
    ///
    /// # Returns
    /// A vector of `Plan` instances representing execution configurations for the instances and
    /// tables.
    ///
    /// # Panics
    /// Panics if any counter cannot be downcasted to an `BinaryCounter`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let mut total_add = 0;
        let mut total_basic_wo_add = 0;
        let mut total_extension = 0;
        let binary_counters: Vec<(ChunkId, &BinaryCounterInputGen)> = counters
            .iter()
            .map(|(chunk_id, counter)| {
                let counter =
                    Metrics::as_any(&**counter).downcast_ref::<BinaryCounterInputGen>().unwrap();
                total_add += counter.counter_add.inst_count as usize;
                total_basic_wo_add += counter.counter_basic_wo_add.inst_count as usize;
                total_extension += counter.counter_extension.inst_count as usize;
                (*chunk_id, counter)
            })
            .collect();

        let extension_counters: Vec<(ChunkId, usize)> = binary_counters
            .iter()
            .map(|(id, c)| (id.clone(), c.counter_extension.inst_count as usize))
            .collect();
        let mut binary_plans: Vec<Plan> = self.plan_for_extensions(extension_counters);

        // TODO: complex solutions for addings use padding of basic binary, to safe
        // at maximum one instance.

        let size_without_adds = Self::size_basic_of(total_add + total_basic_wo_add);
        let size_on_add =
            Self::size_basic_of(total_basic_wo_add) + Self::size_basic_add_of(total_add);

        if size_on_add < size_without_adds {
            let add_counters = binary_counters
                .iter()
                .map(|(id, c)| (*id, c.counter_add.inst_count as usize))
                .collect();
            binary_plans.append(&mut self.plan_for_adds(add_counters));
            let basic_counters = binary_counters
                .iter()
                .map(|(id, c)| (*id, c.counter_basic_wo_add.inst_count as usize))
                .collect();
            binary_plans.append(&mut self.plan_for_basics(basic_counters, false));
        } else {
            let basic_counters = binary_counters
                .iter()
                .map(|(id, c)| {
                    (*id, (c.counter_basic_wo_add.inst_count + c.counter_add.inst_count) as usize)
                })
                .collect();
            binary_plans.append(&mut self.plan_for_basics(basic_counters, true));
        };

        binary_plans
    }
}
