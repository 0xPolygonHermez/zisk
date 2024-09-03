use std::{error::Error, sync::Arc, path::PathBuf};

use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};
use proofman::{WitnessLibrary, WitnessManager};
use pil_std_lib::Std;

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;

use crate::{RangeCheck, Pilout};

pub struct RangeCheckWitness<F> {
    pub wcm: WitnessManager<F>,
    pub range_check: Arc<RangeCheck<F>>,
    pub std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> RangeCheckWitness<F> {
    pub fn new() -> Self {
        let mut wcm = WitnessManager::new();

        let std_lib = Std::new(&mut wcm, None);
        let range_check = RangeCheck::new(&mut wcm, std_lib.clone());

        RangeCheckWitness { wcm, range_check, std_lib }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for RangeCheckWitness<F> {
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        // Execute those components that need to be executed
        self.range_check.execute(pctx, ectx, sctx);
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
    let range_check_witness = RangeCheckWitness::new();
    Ok(Box::new(range_check_witness))
}
