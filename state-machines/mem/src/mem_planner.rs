use std::sync::Arc;

use sm_common::{BusDeviceMetrics, ChunkId, Plan, Planner};
use zisk_pil::{INPUT_DATA_AIR_IDS, MEM_AIR_IDS, ROM_DATA_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{
    MemAlignPlanner, MemCounters, MemModulePlanner, INPUT_DATA_W_ADDR_END, INPUT_DATA_W_ADDR_INIT,
    RAM_W_ADDR_END, RAM_W_ADDR_INIT, ROM_DATA_W_ADDR_END, ROM_DATA_W_ADDR_INIT,
};

pub trait MemPlanCalculator {
    fn plan(&mut self);
    fn collect_plans(&mut self) -> Vec<Plan>;
}

#[derive(Default)]
pub struct MemPlanner {}

impl MemPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

impl Planner for MemPlanner {
    fn plan(&self, metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // convert generic information to specific information
        let _counters: Vec<(ChunkId, &MemCounters)> = metrics
            .iter()
            .map(|(chunk_id, metric)| {
                (*chunk_id, metric.as_any().downcast_ref::<MemCounters>().unwrap())
            })
            .collect();

        let counters = Arc::new(_counters);
        let mut planners: Vec<Box<dyn MemPlanCalculator>> = vec![
            Box::new(MemModulePlanner::new(
                ZISK_AIRGROUP_ID,
                MEM_AIR_IDS[0],
                RAM_W_ADDR_INIT,
                RAM_W_ADDR_END,
                counters.clone(),
            )),
            Box::new(MemModulePlanner::new(
                ZISK_AIRGROUP_ID,
                ROM_DATA_AIR_IDS[0],
                ROM_DATA_W_ADDR_INIT,
                ROM_DATA_W_ADDR_END,
                counters.clone(),
            )),
            Box::new(MemModulePlanner::new(
                ZISK_AIRGROUP_ID,
                INPUT_DATA_AIR_IDS[0],
                INPUT_DATA_W_ADDR_INIT,
                INPUT_DATA_W_ADDR_END,
                counters.clone(),
            )),
            Box::new(MemAlignPlanner::new(counters.clone())),
        ];
        for item in &mut planners {
            item.plan();
        }
        let mut plans: Vec<Plan> = Vec::new();
        for item in &mut planners {
            plans.append(&mut item.collect_plans());
        }
        plans
    }
}
