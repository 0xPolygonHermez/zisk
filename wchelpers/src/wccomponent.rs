use common::{ExecutionCtx, ProofCtx};

pub trait WCComponent<F> {
    fn start_proof(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {}

    fn end_proof(&self) {}

    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx);
}
