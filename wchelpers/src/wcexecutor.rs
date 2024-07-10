use common::{ExecutionCtx, ProofCtx};

pub trait WCExecutor<F> {
    fn execute(&self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx);
}
