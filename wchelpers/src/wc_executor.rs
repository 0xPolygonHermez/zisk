use common::{ExecutionCtx, ProofCtx};

pub trait WCExecutor<F> {
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx);
}
