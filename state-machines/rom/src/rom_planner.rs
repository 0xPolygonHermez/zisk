use sm_common::{BusDeviceMetrics, CheckPointType, ChunkId, InstanceType, Metrics, Plan, Planner};
use zisk_common::ROM_BUS_ID;
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::RomCounter;

pub struct RomPlanner {}

impl Planner for RomPlanner {
    fn plan(&self, metrics: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        if metrics.is_empty() {
            panic!("RomPlanner::plan() No metrics found");
        }

        let mut total = RomCounter::new(ROM_BUS_ID);

        for (_, metric) in metrics {
            let metric = metric.as_any().downcast_ref::<RomCounter>().unwrap();
            total.add(metric);
        }

        vec![Plan::new(
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            None,
            InstanceType::Instance,
            CheckPointType::None,
            Some(Box::new(total)),
        )]
    }
}
