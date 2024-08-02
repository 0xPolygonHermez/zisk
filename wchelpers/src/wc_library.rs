use common::{ExecutionCtx, ProofCtx, WCPilout};
use common::Prover;

pub trait WCLibrary<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx);
    fn end_proof(&mut self);
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx);
    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx);
    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        provers: &Vec<Box<dyn Prover<F>>>,
    );

    fn pilout(&self) -> WCPilout;
}
