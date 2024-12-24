use sm_common::{CheckPoint, CollectInfoSkip, InstanceType, Plan};
use zisk_pil::{MAIN_AIR_IDS, ZISK_AIRGROUP_ID};
use ziskemu::EmuTrace;

pub struct MainPlanner {}

impl MainPlanner {
    pub fn plan(min_traces: &[EmuTrace]) -> Vec<Plan> {
        (0..min_traces.len())
            .map(|segment_id| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    MAIN_AIR_IDS[0],
                    Some(segment_id),
                    InstanceType::Instance,
                    CheckPoint::Single(segment_id),
                    Some(Box::new(CollectInfoSkip::new(0))),
                    None,
                )
            })
            .collect()
    }
}
