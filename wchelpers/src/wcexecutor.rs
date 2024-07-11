use common::{ExecutionCtx, ProofCtx};

pub trait WCExecutor {
    fn execute(&self, pctx: &mut ProofCtx, ectx: &mut ExecutionCtx);
}
