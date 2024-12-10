use p3_field::PrimeField;
use sm_common::{plan, ChunkId, InstCount, InstanceType, Metrics, Plan, Planner};
use zisk_pil::{
    BinaryExtensionTrace, BinaryTrace, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS,
    BINARY_EXTENSION_TABLE_AIR_IDS, BINARY_TABLE_AIR_IDS, ZISK_AIRGROUP_ID,
};

use crate::BinaryCounter;

pub struct BinaryPlanner<F: PrimeField> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField> Default for BinaryPlanner<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> BinaryPlanner<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

impl<F: PrimeField> Planner for BinaryPlanner<F> {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn Metrics>)>) -> Vec<Plan> {
        // Prepare counts for binary
        let (count_binary, count_binary_e): (Vec<_>, Vec<_>) = counters
            .iter()
            .map(|(chunk_id, counter)| {
                let binary_counter = counter.as_any().downcast_ref::<BinaryCounter>().unwrap();
                (
                    InstCount::new(*chunk_id, binary_counter.binary.inst_count as u64),
                    InstCount::new(*chunk_id, binary_counter.binary_extension.inst_count as u64),
                )
            })
            .collect();

        let binaries = [
            (BINARY_AIR_IDS[0], BinaryTrace::<F>::NUM_ROWS, count_binary),
            (BINARY_EXTENSION_AIR_IDS[0], BinaryExtensionTrace::<F>::NUM_ROWS, count_binary_e),
        ];

        let mut plan_result = Vec::new();
        for (air_id, num_rows, counts) in binaries.iter() {
            let plan: Vec<_> = plan(counts, *num_rows as u64)
                .into_iter()
                .map(|checkpoint| {
                    Plan::new(
                        ZISK_AIRGROUP_ID,
                        *air_id,
                        None,
                        InstanceType::Instance,
                        Some(checkpoint),
                        None,
                    )
                })
                .collect();

            plan_result.extend(plan);
        }

        for &air_id in &[BINARY_TABLE_AIR_IDS[0], BINARY_EXTENSION_TABLE_AIR_IDS[0]] {
            plan_result.push(Plan::new(
                ZISK_AIRGROUP_ID,
                air_id,
                None,
                InstanceType::Table,
                None,
                None,
            ));
        }

        plan_result
    }
}
