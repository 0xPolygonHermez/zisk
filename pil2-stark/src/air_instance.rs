use std::collections::HashMap;

use crate::{ExecutionCtx, ProofCtx};

#[allow(dead_code)]
pub struct AirInstanceId {
    airgroup_id: i32,
    air_id: i32,
    instance_id: i32,
}

pub enum AirInstancesSet {
    All,
    Set(Vec<AirInstanceId>),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AirInstance {
    pub airgroup_id: i32,
    pub air_id: i32,
    pub instance_id: i32,
    pub meta: Option<Box<dyn std::any::Any>>,
}

pub trait AirInstanceWitnessComputation<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx);

    fn end_proof(&self, proof_ctx: &ProofCtx<F>);

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance);
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AirInstanceMap {
    pub inner: HashMap<i32, HashMap<i32, AirInstance>>,
}

impl AirInstanceMap {
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }
}
