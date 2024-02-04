use crate::executor::ExecutorBase;
use crate::proof_ctx::ProofCtx;
use crate::config::Config;

// WITNESS CALCULATOR MANAGER
// ================================================================================================
pub trait WitnessCalculatorManager<T> {
    const MY_NAME: &'static str;

    fn new(wc: Vec<Box<dyn ExecutorBase<T>>>) -> Self;
    fn witness_computation(&self, stage_id: u32, config: &Box<dyn Config>, proof_ctx: &mut ProofCtx<T>);
}
