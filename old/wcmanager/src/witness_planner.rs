use common::AirGroupInstanceMap;

pub trait WitnessPlanner<F> {
    fn calculate_air_instances_map(&self, proof_ctx: &AirGroupInstanceMap);
}

pub struct DefaultWitnessPlanner {}

impl DefaultWitnessPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(unused_variables)]
impl<F> WitnessPlanner<F> for DefaultWitnessPlanner {
    fn calculate_air_instances_map(&self, proof_ctx: &AirGroupInstanceMap) {}
}
