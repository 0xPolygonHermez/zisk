use proofman_common::*;

pub struct MemSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F> MemSM<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}
#[allow(dead_code)]
pub struct MemSMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for MemSM<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        unimplemented!()
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        unimplemented!()
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        unimplemented!()
    }
}
