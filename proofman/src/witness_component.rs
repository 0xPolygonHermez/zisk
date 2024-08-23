use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;

pub trait WitnessComponent<F> {
    fn start_proof(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx, _sctx: &SetupCtx) {}

    fn end_proof(&self) {}

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: usize,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    );
}
