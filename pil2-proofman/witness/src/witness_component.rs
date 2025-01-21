use std::sync::Arc;

use proofman_common::{ProofCtx, SetupCtx};

pub trait WitnessComponent<F>: Send + Sync {
    fn start_proof(&self, _pctx: Arc<ProofCtx<F>>, _sctx: Arc<SetupCtx>) {}

    fn execute(&self, _pctx: Arc<ProofCtx<F>>) {}

    fn debug(&self, _pctx: Arc<ProofCtx<F>>) {}

    fn calculate_witness(&self, _stage: u32, _pctx: Arc<ProofCtx<F>>, _sctx: Arc<SetupCtx>) {}

    fn end_proof(&self) {}
}
