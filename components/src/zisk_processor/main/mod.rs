use pil2_stark::*;

pub struct MainSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F> MainSM<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
}
#[allow(dead_code)]
pub struct MainSMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for MainSM<F> {
    fn start_proof(&self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {}

    fn end_proof(&self, proof_ctx: &ProofCtx<F>) {}

    fn calculate_witness(&self, stage: u32, proof_ctx: &ProofCtx<F>, air_instance: &AirInstance) {
        unimplemented!()
    }
}

#[allow(unused_variables)]
impl<F> WitnessExecutor<F> for MainSM<F> {
    fn start_execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
        unimplemented!()
    }

    fn execute(&mut self, proof_ctx: &ProofCtx<F>, execution_ctx: &ExecutionCtx) {
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
        unimplemented!()
    }
}
