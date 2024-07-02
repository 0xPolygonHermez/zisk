use log::trace;
use pil2_stark::*;

pub struct MainSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F> MainSM<F> {
    const MY_NAME: &'static str = "MainSM  ";

    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}
#[allow(dead_code)]
pub struct MainSMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for MainSM<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("{}: ··· Starting proof", Self::MY_NAME);
    }

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {
        trace!("Ending proof for MainSM");
    }

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        trace!("Calculating witness for MainSM at stage {}", stage);
    }
}

#[allow(unused_variables)]
impl<F> WitnessExecutor<F> for MainSM<F> {
    fn start_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("Starting execution for MainSM");
    }

    fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("Executing for MainSM");
        // arith.startExecute(ctx, ectx);
        // bin.starteExecute(ctx, ectx);
        // let mainCtx = proofs.get(ctx.idProof);
        // ..
        // ..
        // arith.mul(ctx, a, b)
        // ..
        // ..
        // arith.endExecute(ctx);
        // bin.endExecute(ctx);
    }

    fn end_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        trace!("Ending execution for MainSM");
    }
}
