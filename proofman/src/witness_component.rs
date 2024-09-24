use std::sync::Arc;

use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx};

pub trait WitnessComponent<F>: Send + Sync {
    fn start_proof(&self, _pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx>, _sctx: Arc<SetupCtx>) {}

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: Option<usize>,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    );

    fn end_proof(&self) {}
}
