use p3_field::PrimeField;
use sm_common::{plan, ChunkId, InstCount, InstanceType, Metrics, Plan, Planner, RegularCounter};
use zisk_pil::{
    ArithTrace, ARITH_AIR_IDS, ARITH_RANGE_TABLE_AIR_IDS, ARITH_TABLE_AIR_IDS, ZISK_AIRGROUP_ID,
};

pub struct ArithPlanner<F: PrimeField> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: PrimeField> Default for ArithPlanner<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> ArithPlanner<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

impl<F: PrimeField> Planner for ArithPlanner<F> {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn Metrics>)>) -> Vec<Plan> {
        // Prepare counts for arith
        let count_arith: Vec<_> = counters
            .iter()
            .map(|(chunk_id, counter)| {
                let arith_counter = counter.as_any().downcast_ref::<RegularCounter>().unwrap();
                InstCount::new(*chunk_id, arith_counter.inst_count())
            })
            .collect();

        let mut plan_result: Vec<_> = plan(&count_arith, ArithTrace::<F>::NUM_ROWS as u64)
            .into_iter()
            .map(|checkpoint| {
                Plan::new(
                    ZISK_AIRGROUP_ID,
                    ARITH_AIR_IDS[0],
                    None,
                    InstanceType::Instance,
                    Some(checkpoint),
                    None,
                )
            })
            .collect();

        for &air_id in &[ARITH_TABLE_AIR_IDS[0], ARITH_RANGE_TABLE_AIR_IDS[0]] {
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
