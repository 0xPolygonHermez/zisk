use crate::{AirInstanceMap, ProofCtx};

pub trait WitnessPlanner<F> {
    fn get_air_instances_map(&self, proof_ctx: &ProofCtx<F>) -> AirInstanceMap;
}

pub struct DefaultWitnessPlanner {}

impl DefaultWitnessPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(unused_variables)]
impl<F> WitnessPlanner<F> for DefaultWitnessPlanner {
    fn get_air_instances_map(&self, proof_ctx: &ProofCtx<F>) -> AirInstanceMap {
        AirInstanceMap::new()
    }
}
