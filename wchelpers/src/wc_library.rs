use common::{ExecutionCtx, ProofCtx, WCPilOut};
use common::Prover;

pub trait WCLibrary<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx);
    fn end_proof(&mut self);
    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx);
    fn initialize_air_instances(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx);
    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    );

    fn get_pilout(&self) -> WCPilOut;
}
