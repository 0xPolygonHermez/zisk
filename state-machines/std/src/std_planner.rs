use std::sync::Arc;

use p3_field::PrimeField;
use pil_std_lib::Std;
use sm_common::{ChunkId, Metrics, Plan, Planner};

pub struct StdPlanner<F: PrimeField> {
    std: Arc<Std<F>>,
}

impl<F: PrimeField> StdPlanner<F> {
    pub fn new(std: Arc<Std<F>>) -> Self {
        Self { std }
    }
}

impl<F: PrimeField> Planner for StdPlanner<F> {
    fn plan(&self, _: Vec<(ChunkId, Box<dyn Metrics>)>) -> Vec<Plan> {
        self.std
            .get_ranges()
            .into_iter()
            .map(|(airgroup_id, air_id, rc_type)| {
                Plan::new(airgroup_id, air_id, None, None, Some(Box::new(rc_type)))
            })
            .collect()
    }
}
