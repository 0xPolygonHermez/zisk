use std::sync::Arc;

use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

pub trait WitnessComponent<F>: Send + Sync {
    fn start_proof(&self, _pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx>, _sctx: Arc<SetupCtx>) {}

    fn calculate_witness(
        &self,
        _stage: u32,
        _air_instance: Option<usize>,
        _pctx: Arc<ProofCtx<F>>,
        _ectx: Arc<ExecutionCtx>,
        _sctx: Arc<SetupCtx>,
    ) {
    }

    fn end_proof(&self) {}
}
