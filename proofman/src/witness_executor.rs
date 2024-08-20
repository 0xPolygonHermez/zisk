use proofman_common::{ExecutionCtx, ProofCtx};

pub trait WitnessExecutor<F> {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx);
}
