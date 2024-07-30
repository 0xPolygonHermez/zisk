use common::{AirInstance, ExecutionCtx, ProofCtx, Prover};

pub trait WCComponent<F> {
    fn start_proof(&self, _pctx: &ProofCtx<F>, _ectx: &ExecutionCtx) {}

    fn end_proof(&self) {}

    fn calculate_plan(&self, _ectx: &mut ExecutionCtx);

    fn calculate_witness(
        &self,
        stage: u32,
        air_instance: &AirInstance,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    );
}
