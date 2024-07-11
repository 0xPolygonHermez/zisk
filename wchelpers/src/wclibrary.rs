use common::{ExecutionCtx, ProofCtx, WitnessPilOut};

pub trait WCLibrary {
    fn start_proof(&mut self, pctx: &mut ProofCtx, ectx: &mut ExecutionCtx);
    fn end_proof(&mut self);
    fn calculate_plan(&mut self, ectx: &mut ExecutionCtx);
    fn initialize_air_instances(&mut self, pctx: &mut ProofCtx, ectx: &ExecutionCtx);
    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx, ectx: &ExecutionCtx);

    fn get_pilout(&self) -> WitnessPilOut;
}
