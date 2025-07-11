//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! binary operations (basic, extensions and dedicated adds)
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging binary operation counts by operation and metadata to construct detailed plans.

use std::any::Any;

use crate::BinaryCounter;
use proofman_common::PreCalculate;
use zisk_common::{
    plan, BusDeviceMetrics, CheckPoint, ChunkId, InstCount, InstanceType, Metrics, Plan, Planner,
};
use zisk_pil::{
    BinaryAddTrace, BinaryExtensionTableTrace, BinaryExtensionTrace, BinaryTableTrace, BinaryTrace,
};

/// The `BinaryPlanner` struct organizes execution plans for binaries instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct BinaryPlanner {}

impl BinaryPlanner {
    pub fn new() -> Self {
        Self {}
    }
    fn size_basic_of(rows: usize) -> usize {
        if rows == 0 {
            0
        } else {
            ((rows - 1 / BinaryTrace::<usize>::NUM_ROWS) + 1) * BinaryTrace::<usize>::ROW_SIZE
        }
    }

    fn size_basic_add_of(rows: usize) -> usize {
        if rows == 0 {
            0
        } else {
            ((rows - 1 / BinaryAddTrace::<usize>::NUM_ROWS) + 1) * BinaryAddTrace::<usize>::ROW_SIZE
        }
    }

    fn plan_for_extensions(&self, counters: &Vec<(ChunkId, &BinaryCounter)>) -> Vec<Plan> {
        let extension_counters: Vec<InstCount> = counters
            .iter()
            .map(|(chunk_id, c)| InstCount::new(*chunk_id, c.counter_extension.inst_count))
            .collect();

        let extension_num_rows = BinaryExtensionTrace::<usize>::NUM_ROWS;

        let mut plans: Vec<_> = plan(&extension_counters, extension_num_rows as u64)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let converted: Box<dyn Any> = Box::new(collect_info);
                Plan::new(
                    BinaryExtensionTrace::<usize>::AIRGROUP_ID,
                    BinaryExtensionTrace::<usize>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    PreCalculate::Fast,
                    Some(converted),
                )
            })
            .collect();

        if !plans.is_empty() {
            plans.push(Plan::new(
                BinaryExtensionTableTrace::<usize>::AIRGROUP_ID,
                BinaryExtensionTableTrace::<usize>::AIR_ID,
                None,
                InstanceType::Table,
                CheckPoint::None,
                PreCalculate::None,
                None,
            ));
        }
        plans
    }
    fn plan_for_basics(
        &self,
        counters: &Vec<(ChunkId, &BinaryCounter)>,
        with_adds: bool,
    ) -> Vec<Plan> {
        let basic_counters: Vec<InstCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                InstCount::new(
                    *chunk_id,
                    c.counter_basic_wo_add.inst_count
                        + if with_adds { c.counter_add.inst_count } else { 0 },
                )
            })
            .collect();

        let basic_num_rows = BinaryTrace::<usize>::NUM_ROWS;

        let mut plans: Vec<_> = plan(&basic_counters, basic_num_rows as u64)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let converted: Box<dyn Any> = Box::new((with_adds, collect_info));
                Plan::new(
                    BinaryTrace::<usize>::AIRGROUP_ID,
                    BinaryTrace::<usize>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    PreCalculate::Fast,
                    Some(converted),
                )
            })
            .collect();

        if !plans.is_empty() {
            plans.push(Plan::new(
                BinaryTableTrace::<usize>::AIRGROUP_ID,
                BinaryTableTrace::<usize>::AIR_ID,
                None,
                InstanceType::Table,
                CheckPoint::None,
                PreCalculate::None,
                None,
            ));
        }
        plans
    }

    fn plan_for_adds(&self, counters: &Vec<(ChunkId, &BinaryCounter)>) -> Vec<Plan> {
        let add_counters: Vec<InstCount> = counters
            .iter()
            .map(|(chunk_id, c)| InstCount::new(*chunk_id, c.counter_add.inst_count))
            .collect();

        let add_num_rows = BinaryAddTrace::<usize>::NUM_ROWS;

        plan(&add_counters, add_num_rows as u64)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let converted: Box<dyn Any> = Box::new(collect_info);
                Plan::new(
                    BinaryAddTrace::<usize>::AIRGROUP_ID,
                    BinaryAddTrace::<usize>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    PreCalculate::Fast,
                    Some(converted),
                )
            })
            .collect()
    }
}

impl Planner for BinaryPlanner {
    /// Generates execution plans for binary instances and tables.
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

        let binary_counters: Vec<(ChunkId, &BinaryCounter)> = counters
            .iter()
            .map(|(chunk_id, counter)| {
                let counter = Metrics::as_any(&**counter).downcast_ref::<BinaryCounter>().unwrap();
                total_add += counter.counter_add.inst_count as usize;
                total_basic_wo_add += counter.counter_basic_wo_add.inst_count as usize;
                total_extension += counter.counter_extension.inst_count as usize;
                (*chunk_id, counter)
            })
            .collect();

        let mut plans = self.plan_for_extensions(&binary_counters);

        let size_without_adds = Self::size_basic_of(total_add + total_basic_wo_add);
        let size_on_add =
            Self::size_basic_of(total_basic_wo_add) + Self::size_basic_add_of(total_add);

        let enable_bin_add_sm = size_on_add < size_without_adds;

        if enable_bin_add_sm {
            let mut add_plans = self.plan_for_adds(&binary_counters);
            plans.append(&mut add_plans);
        }

        let mut basic_plans = self.plan_for_basics(&binary_counters, !enable_bin_add_sm);
        plans.append(&mut basic_plans);

        plans
    }
}
