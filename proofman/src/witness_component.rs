use proofman_common::{ExecutionCtx, ProofCtx};
use proofman_setup::SetupCtx;
use rayon::Scope;

pub trait WitnessComponent<F> {
    fn start_proof(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx, _sctx: &SetupCtx) {}

    fn register_predecessor(&self) {}

    fn unregister_predecessor(&self, _scope: &Scope) {}

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: Option<usize>,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    );

    fn end_proof(&self) {}
}
