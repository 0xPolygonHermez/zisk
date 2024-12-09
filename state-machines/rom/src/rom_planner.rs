use sm_common::{ChunkId, Metrics, Plan, Planner};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::RomCounter;

pub struct RomPlanner {}

impl Planner for RomPlanner {
    fn plan(&self, metrics: Vec<(ChunkId, Box<dyn Metrics>)>) -> Vec<Plan> {
        if metrics.is_empty() {
            panic!("RomPlanner::plan() found no metrics");
        }

        let mut total = RomCounter::default();

        for (_, metric) in metrics {
            let metric = metric.as_any().downcast_ref::<RomCounter>().unwrap();
            total.add(metric);
        }

        vec![Plan::new(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0], None, None, Some(Box::new(total)))]
    }
}
