use std::sync::Arc;

use proofman_common::{ExecutionCtx, ProofCtx, WitnessPilout};
use p3_field::AbstractField;
use p3_goldilocks::Goldilocks;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_setup::SetupCtx;

use std::error::Error;
use std::path::PathBuf;

use crate::{FibonacciSquare, Pilout, Module};

pub struct FibonacciVadcop<F> {
    pub wcm: WitnessManager<F>,
    pub fibonacci: Arc<FibonacciSquare>,
    pub module: Arc<Module>,
}

impl<F: AbstractField + Copy> FibonacciVadcop<F> {
    pub fn new() -> Self {
        let mut wcm = WitnessManager::new();

        let module = Module::new(&mut wcm);
        let fibonacci = FibonacciSquare::new(&mut wcm, module.clone());

        FibonacciVadcop { wcm, fibonacci, module }
    }
}

impl<F: AbstractField + Copy> WitnessLibrary<F> for FibonacciVadcop<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        pctx.public_inputs =
            vec![25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0]; // TODO: NOT SHOULD BE HARDCODED!
        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        self.fibonacci.execute(pctx, ectx, sctx);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    _public_inputs_path: PathBuf,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();
    let fibonacci_witness = FibonacciVadcop::new();
    Ok(Box::new(fibonacci_witness))
}
