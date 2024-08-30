use std::{error::Error, path::PathBuf};

use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout};
use proofman_setup::SetupCtx;

/// This is the type of the function that is used to load a witness library.
pub type WitnessLibInitFn<F> = fn(Option<PathBuf>, PathBuf) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn Error>>;

pub trait WitnessLibrary<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx);

    fn end_proof(&mut self);
    
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx);

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx);

    fn pilout(&self) -> WitnessPilout;
}
