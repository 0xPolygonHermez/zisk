use std::{error::Error, path::PathBuf};

use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout};

/// This is the type of the function that is used to load a witness library.
pub type WitnessLibInitFn<F> =
    fn(Option<PathBuf>, PathBuf, PathBuf) -> Result<Box<dyn WitnessLibrary<F>>, Box<dyn Error>>;

pub trait WitnessLibrary<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx);
    fn end_proof(&mut self);
    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx);
    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx);

    fn pilout(&self) -> WitnessPilout;
}
