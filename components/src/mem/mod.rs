use log::trace;
use pil2_stark::*;

pub struct MemSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F> MemSM<F> {
    const MY_NAME: &'static str = "MemSM   ";

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}
#[allow(dead_code)]
pub struct MemSMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for MemSM<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for MemSM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for MemSM at stage {}", stage);
    }
}
