use std::sync::Arc;

#[cfg(feature = "debug_mem")]
use crate::MemHelpers;

use sm_common::{BusDeviceMetrics, ChunkId, Plan, Planner};
use zisk_pil::{
    InputDataTrace, MemTrace, RomDataTrace, INPUT_DATA_AIR_IDS, MEM_AIR_IDS, ROM_DATA_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

use crate::{
    MemAlignPlanner, MemCounters, MemModulePlanner, MemModulePlannerConfig, INPUT_DATA_W_ADDR_END,
    INPUT_DATA_W_ADDR_INIT, RAM_W_ADDR_END, RAM_W_ADDR_INIT, ROM_DATA_W_ADDR_END,
    ROM_DATA_W_ADDR_INIT,
};

#[cfg(feature = "debug_mem")]
use crate::{MemDebug, MEM_BYTES};

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

    #[cfg(feature = "debug_mem")]
    fn collect_debug_data(&self, counters: Arc<Vec<(ChunkId, &MemCounters)>>) {
        let mut debug = MemDebug::new();
        let mut i = 0;
        while i < counters.len() {
            debug.add(&counters[i].1.debug);
            i += 1;
        }
        if !debug.is_empty() {
            debug.save_to_file("/tmp/mem_debug.txt");
        }
    }

    #[cfg(feature = "debug_mem")]
    fn debug_plans(&self, plans: &[Plan]) {
        use log::info;

        use crate::MemModuleSegmentCheckPoint;

        for (index, plan) in plans.iter().enumerate() {
            if plan.air_id == MEM_AIR_IDS[0] ||
                plan.air_id == INPUT_DATA_AIR_IDS[0] ||
                plan.air_id == ROM_DATA_AIR_IDS[0]
            {
                let meta = plan
                    .meta
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<MemModuleSegmentCheckPoint>()
                    .unwrap();
                info!(
                    "[Mem] PLAN #{} [{}:{}:{}] {:?} [0x{:X},{}] => [0x{:X},{}] skip:{} last:{}",
                    index,
                    plan.airgroup_id,
                    plan.air_id,
                    plan.segment_id.unwrap_or(0),
                    plan.check_point,
                    MemHelpers::get_addr(meta.prev_addr),
                    meta.prev_step,
                    MemHelpers::get_addr(meta.last_addr),
                    meta.last_step,
                    meta.skip_rows,
                    meta.is_last_segment,
                );
            } else {
                info!(
                    "[Mem] PLAN #{} [{}:{}:{}] {:?}",
                    index,
                    plan.airgroup_id,
                    plan.air_id,
                    plan.segment_id.unwrap_or(0),
                    plan.check_point,
                );
            }
        }
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

        #[cfg(feature = "debug_mem")]
        self.collect_debug_data(counters.clone());

        let mut planners: Vec<Box<dyn MemPlanCalculator>> = vec![
            Box::new(MemModulePlanner::new(
                MemModulePlannerConfig {
                    airgroup_id: ZISK_AIRGROUP_ID,
                    air_id: MEM_AIR_IDS[0],
                    from_addr: RAM_W_ADDR_INIT,
                    to_addr: RAM_W_ADDR_END,
                    rows: MemTrace::<usize>::NUM_ROWS as u32,
                    consecutive_addr: false,
                    intermediate_step_reads: true,
                    map_registers: true,
                },
                counters.clone(),
            )),
            Box::new(MemModulePlanner::new(
                MemModulePlannerConfig {
                    airgroup_id: ZISK_AIRGROUP_ID,
                    air_id: ROM_DATA_AIR_IDS[0],
                    from_addr: ROM_DATA_W_ADDR_INIT,
                    to_addr: ROM_DATA_W_ADDR_END,
                    rows: RomDataTrace::<usize>::NUM_ROWS as u32,
                    consecutive_addr: true,
                    intermediate_step_reads: false,
                    map_registers: false,
                },
                counters.clone(),
            )),
            Box::new(MemModulePlanner::new(
                MemModulePlannerConfig {
                    airgroup_id: ZISK_AIRGROUP_ID,
                    air_id: INPUT_DATA_AIR_IDS[0],
                    from_addr: INPUT_DATA_W_ADDR_INIT,
                    to_addr: INPUT_DATA_W_ADDR_END,
                    rows: InputDataTrace::<usize>::NUM_ROWS as u32,
                    consecutive_addr: true,
                    intermediate_step_reads: false,
                    map_registers: false,
                },
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
        #[cfg(feature = "debug_mem")]
        self.debug_plans(plans.as_ref());
        plans
    }
}
