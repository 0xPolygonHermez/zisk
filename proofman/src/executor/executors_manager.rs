use crate::executor::Executor;
use crate::proof_manager_config::Config;
use crate::proof_ctx::ProofCtx;
use log::debug;

// WITNESS CALCULATOR MANAGER TRAIT
// ================================================================================================
pub trait ExecutorsManager<T> {
    const MY_NAME: &'static str;

    fn new(wc: Vec<Box<dyn Executor<T>>>) -> Self;
    fn witness_computation(&self, config: &dyn Config, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}

// WITNESS CALCULATOR MANAGER (SEQUENTIAL)
// ================================================================================================
pub struct ExecutorsManagerSequential<T> {
    wc: Vec<Box<dyn Executor<T>>>,
}

impl<T> ExecutorsManager<T> for ExecutorsManagerSequential<T> {
    const MY_NAME: &'static str = "exectrsm";

    fn new(wc: Vec<Box<dyn Executor<T>>>) -> Self {
        debug!("{}: Initializing...", Self::MY_NAME);

        ExecutorsManagerSequential { wc }
    }

    fn witness_computation(&self, config: &dyn Config, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        debug!("{}: --> Computing witness stage {}", Self::MY_NAME, stage_id);

        for wc in self.wc.iter() {
            wc.witness_computation(config, stage_id, proof_ctx);
        }

        debug!("{}: <-- Computing witness stage {}", Self::MY_NAME, stage_id);
    }
}
