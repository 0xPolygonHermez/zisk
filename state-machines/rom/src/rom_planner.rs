use sm_common::{CheckPoint, ChunkId, Plan, Planner, Surveyor};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::RomSurveyor;

pub struct RomPlanner {}

impl Planner for RomPlanner {
    fn plan(&self, surveys: Vec<(ChunkId, Box<dyn Surveyor>)>) -> Vec<Plan> {
        if surveys.len() == 0 {
            panic!("RomPlanner::plan() found no surveys");
        }

        let mut total = RomSurveyor::default();

        for (_, survey) in surveys {
            let survey = survey.as_any().downcast_ref::<RomSurveyor>().unwrap();
            total.add(survey);
        }

        vec![Plan::new(
            ZISK_AIRGROUP_ID,
            ROM_AIR_IDS[0],
            None,
            CheckPoint::new(0, 0),
            Some(Box::new(total)),
        )]
    }
}
