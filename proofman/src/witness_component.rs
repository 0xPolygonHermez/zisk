use proofman_common::{ExecutionCtx, ProofCtx};

pub trait WitnessComponent<F> {
    fn start_proof(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {}

    fn end_proof(&self) {}

    fn calculate_witness(&self, stage: u32, air_instance: usize, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx);
}
