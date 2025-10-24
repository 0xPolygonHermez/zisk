//! The `BinaryPlanner` module defines a planner for generating execution plans specific to
//! binary operations (basic, extensions and dedicated adds)
//!
//! It organizes execution plans for both regular instances and table instances,
//! leveraging binary operation counts by operation and metadata to construct detailed plans.

use crate::BinaryCounter;
use fields::PrimeField64;
use std::any::Any;
use zisk_common::{
    plan_with_frops, BusDeviceMetrics, ChunkId, InstFropsCount, InstanceType, Metrics, Plan,
    Planner,
};
use zisk_pil::{BinaryAddTrace, BinaryExtensionTrace, BinaryTrace};

/// The `BinaryPlanner` struct organizes execution plans for binaries instances and tables.
///
/// It allows adding metadata about instances and tables and generates plans
/// based on the provided counters.
#[derive(Default)]
pub struct BinaryPlanner<F> {
    _marker: std::marker::PhantomData<F>,
}

impl<F: PrimeField64> BinaryPlanner<F> {
    pub fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }
    fn size_basic_of(rows: usize) -> usize {
        if rows == 0 {
            0
        } else {
            ((rows - 1 / BinaryTrace::<F>::NUM_ROWS) + 1) * BinaryTrace::<F>::ROW_SIZE
        }
    }

    fn size_basic_add_of(rows: usize) -> usize {
        if rows == 0 {
            0
        } else {
            ((rows - 1 / BinaryAddTrace::<F>::NUM_ROWS) + 1) * BinaryAddTrace::<F>::ROW_SIZE
        }
    }

    fn plan_for_extensions(&self, counters: &Vec<(ChunkId, &BinaryCounter)>) -> Vec<Plan> {
        let extension_counters: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                InstFropsCount::new(
                    *chunk_id,
                    c.counter_extension.inst_count,
                    c.counter_extension.frops_count,
                )
            })
            .collect();

        let extension_num_rows = BinaryExtensionTrace::<F>::NUM_ROWS;

        let plans: Vec<_> = plan_with_frops(&extension_counters, extension_num_rows as u64)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let converted: Box<dyn Any> = Box::new(collect_info);
                Plan::new(
                    BinaryExtensionTrace::<F>::AIRGROUP_ID,
                    BinaryExtensionTrace::<F>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(converted),
                )
            })
            .collect();

        plans
    }
    fn plan_for_basics(
        &self,
        counters: &Vec<(ChunkId, &BinaryCounter)>,
        with_adds: bool,
    ) -> Vec<Plan> {
        let basic_counters: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                InstFropsCount::new(
                    *chunk_id,
                    c.counter_basic_wo_add.inst_count
                        + if with_adds { c.counter_add.inst_count } else { 0 },
                    c.counter_basic_wo_add.frops_count
                        + if with_adds { c.counter_add.frops_count } else { 0 },
                )
            })
            .collect();

        let basic_num_rows = BinaryTrace::<F>::NUM_ROWS;

        let plans: Vec<_> = plan_with_frops(&basic_counters, basic_num_rows as u64)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let converted: Box<dyn Any> = Box::new((with_adds, collect_info));
                Plan::new(
                    BinaryTrace::<F>::AIRGROUP_ID,
                    BinaryTrace::<F>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(converted),
                )
            })
            .collect();

        plans
    }

    fn plan_for_adds(&self, counters: &Vec<(ChunkId, &BinaryCounter)>) -> Vec<Plan> {
        let add_counters: Vec<InstFropsCount> = counters
            .iter()
            .map(|(chunk_id, c)| {
                InstFropsCount::new(*chunk_id, c.counter_add.inst_count, c.counter_add.frops_count)
            })
            .collect();

        let add_num_rows = BinaryAddTrace::<F>::NUM_ROWS;

        plan_with_frops(&add_counters, add_num_rows as u64)
            .into_iter()
            .map(|(check_point, collect_info)| {
                let converted: Box<dyn Any> = Box::new(collect_info);
                Plan::new(
                    BinaryAddTrace::<F>::AIRGROUP_ID,
                    BinaryAddTrace::<F>::AIR_ID,
                    None,
                    InstanceType::Instance,
                    check_point,
                    Some(converted),
                )
            })
            .collect()
    }
}

impl<F: PrimeField64> Planner for BinaryPlanner<F> {
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
