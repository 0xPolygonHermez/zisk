use crate::{ExecutionCtx, ProofCtx, WitnessPilOut};

pub enum AirInstancesSet {
    All,
    Set(Vec<AirInstance>),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AirInstance {
    pub air_group_id: i32,
    pub air_id: i32,
    pub instance_id: Option<i32>,
    pub meta: Option<Box<dyn std::any::Any>>,
}

pub trait AirInstanceWitnessComputation<'a, F> {
    fn start_proof(&self, proof_ctx: &mut ProofCtx<F>, execution_ctx: &ExecutionCtx, pilout: &WitnessPilOut);

    fn end_proof(&self, proof_ctx: &ProofCtx<F>);

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance);
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AirGroupInstanceMap {
    pub inner: Vec<Vec<AirInstance>>,
}

impl AirGroupInstanceMap {
    pub fn new(num_air_groups: usize) -> Self {
        Self { inner: (0..num_air_groups).map(|_| Vec::new()).collect() }
    }

    pub fn add_air_instance(&mut self, air_group_id: usize, air_instance: AirInstance) {
        self.inner[air_group_id].push(air_instance);
    }
}
