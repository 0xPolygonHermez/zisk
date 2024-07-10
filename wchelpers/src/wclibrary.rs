use common::{ExecutionCtx, ProofCtx};

pub trait WCLibrary<F> {
    fn start_proof(&mut self, pctx: &ProofCtx<F>, ectx: &ExecutionCtx);
    fn end_proof(&mut self);
    fn calculate_plan(&mut self, pctx: &ProofCtx<F>);
    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx);
}
