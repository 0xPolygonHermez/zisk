use log::trace;
use pil2_stark::*;

pub struct BinarySM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F> BinarySM<F> {
    const MY_NAME: &'static str = "BinarySM";

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}

#[allow(dead_code)]
pub struct BinarySMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for BinarySM<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for BinarySM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for BinarySM at stage {}", stage);
    }
}
