use common::{ExecutionCtx, ProofCtx};

pub trait WCComponent {
    fn start_proof(&self, _pctx: &ProofCtx, _ectx: &ExecutionCtx) {}

    fn end_proof(&self) {}

    fn calculate_witness(&self, stage: u32, pctx: &mut ProofCtx, ectx: &ExecutionCtx);
}
