use std::sync::Arc;

use p3_field::PrimeField;
use proofman::WitnessManager;
use sm_common::{plan, ChunkId, InstCount, Plan, Planner, Metrics};
use zisk_pil::{BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::BinarySurveyor;

pub struct BinaryPlanner<F: PrimeField> {
    wcm: Arc<WitnessManager<F>>,
}

impl<F: PrimeField> BinaryPlanner<F> {
    pub fn new(wcm: Arc<WitnessManager<F>>) -> Self {
        Self { wcm }
    }
}

impl<F: PrimeField> Planner for BinaryPlanner<F> {
    fn plan(&self, surveys: Vec<(ChunkId, Box<dyn Metrics>)>) -> Vec<Plan> {
        // Prepare counts for binary
        let (count_binary, count_binary_e): (Vec<_>, Vec<_>) = surveys
            .iter()
            .map(|(chunk_id, surveyor)| {
                let binary_surveyor = surveyor.as_any().downcast_ref::<BinarySurveyor>().unwrap();
                (
                    InstCount::new(*chunk_id, binary_surveyor.binary.inst_count as u64),
                    InstCount::new(*chunk_id, binary_surveyor.binary_extension.inst_count as u64),
                )
            })
            .collect();

        let pctx = self.wcm.get_pctx();

        let binaries =
            [(BINARY_AIR_IDS[0], count_binary), (BINARY_EXTENSION_AIR_IDS[0], count_binary_e)];

        let mut plan_result = Vec::new();
        for (air_id, counts) in binaries.iter() {
            let rows = pctx.pilout.get_air(ZISK_AIRGROUP_ID, *air_id).num_rows() as u64;
            let plan: Vec<_> = plan(counts, rows)
                .into_iter()
                .map(|checkpoint| Plan::new(ZISK_AIRGROUP_ID, *air_id, None, checkpoint, None))
                .collect();
            plan_result.extend(plan);
        }

        plan_result
    }
}
