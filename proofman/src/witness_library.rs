use std::{error::Error, path::PathBuf, sync::Arc};

use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

/// This is the type of the function that is used to load a witness library.
pub type WitnessLibInitFn<F> =
    fn(Option<PathBuf>, Option<PathBuf>) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn Error>>;

pub trait WitnessLibrary<F> {
    fn start_proof(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>);

    fn end_proof(&mut self);

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>);

    fn calculate_witness(&mut self, stage: u32, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>);

    fn debug(&mut self, _pctx: Arc<ProofCtx<F>>, _ectx: Arc<ExecutionCtx>, _sctx: Arc<SetupCtx>) {}

    fn pilout(&self) -> WitnessPilout;
}
