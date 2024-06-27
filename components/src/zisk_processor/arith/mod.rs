extern crate pil2_stark;

use pil2_stark::*;

pub struct ArithSM<F> {
    _phantom: std::marker::PhantomData<F>,
}

#[allow(dead_code, unused_variables)]
impl<F> ArithSM<F> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }

    fn start(&self) {
        // rangeCheck.start(ctx, ectx);
    }

    fn execute(&self, proof_ctx: &ProofCtx<F>) {
        // arithCtx = proofs.get(ctx.idProof);
        // c = a*b;
        // if (arithCtx.callRageChecks) {
        //     arithCtx.asyncs.push(async rangeCheck.check(ctx. b));
        //     arithCtx.asyncs.push(async rangeCheck.check(ctx. b));
        // }
        // return c;
    }

    fn end() {
        // rangeCheck.start(ctx, ectx);
    }

}

#[allow(dead_code)]
pub struct ArithSMMetadata {}

#[allow(unused_variables)]
impl<F> AirInstanceWitnessComputation<F> for ArithSM<F> {
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
