//! The `BabyJubJubPlanner` module defines a planner for generating execution plans for the
//! BabyJubJub point-add precompile, mirroring the arith_eq planner reduced to one operation.

use std::any::Any;

use crate::BabyJubJubCounterInputGen;

use zisk_common::{
    plan, BusDeviceMetrics, ChunkId, InstCount, InstanceInfo, InstanceType, Metrics, Plan, Planner,
};

/// The `BabyJubJubPlanner` struct organizes execution plans for BabyJubJub instances.
#[derive(Default)]
pub struct BabyJubJubPlanner {
    /// BabyJubJub instances info to be planned.
    instances_info: Vec<InstanceInfo>,
}

impl BabyJubJubPlanner {
    /// Creates a new `BabyJubJubPlanner` with no preconfigured instances.
    pub fn new() -> Self {
        Self { instances_info: Vec::new() }
    }

    /// Adds a BabyJubJub instance to the planner.
    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }
}

impl Planner for BabyJubJubPlanner {
    /// Generates execution plans for BabyJubJub instances.
    ///
    /// # Panics
    /// Panics if any counter cannot be downcast to a `BabyJubJubCounterInputGen`.
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        let mut count: Vec<Vec<InstCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        counters.iter().for_each(|(chunk_id, counter)| {
            let reg_counter =
                Metrics::as_any(&**counter).downcast_ref::<BabyJubJubCounterInputGen>().unwrap();

            for (index, instance_info) in self.instances_info.iter().enumerate() {
                let inst_count = InstCount::new(
                    *chunk_id,
                    reg_counter.inst_count(instance_info.op_type).unwrap(),
                );

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

        plan_result
    }
}
