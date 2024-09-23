use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Pilout, SimpleLeft, SimpleRight};

pub struct SimpleWitness<F: PrimeField> {
    pub wcm: WitnessManager<F>,
    pub simple_left: Arc<SimpleLeft<F>>,
    pub simple_right: Arc<SimpleRight<F>>,
    pub std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> Default for SimpleWitness<F>
where
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> SimpleWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        let mut wcm = WitnessManager::new();

        let std_lib = Std::new(&mut wcm, None);
        let simple_left = SimpleLeft::new(&mut wcm);
        let simple_right = SimpleRight::new(&mut wcm);

        SimpleWitness {
            wcm,
            simple_left,
            simple_right,
            std_lib,
        }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for SimpleWitness<F>
where
    Standard: Distribution<F>,
{
    fn start_proof(&mut self, pctx: &mut ProofCtx<F>, ectx: &ExecutionCtx, sctx: &SetupCtx) {
        self.wcm.start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.end_proof();
    }

    fn execute(&self, pctx: &mut ProofCtx<F>, ectx: &mut ExecutionCtx, sctx: &SetupCtx) {
        // Execute those components that need to be executed
        self.simple_left.execute(pctx, ectx, sctx);
        self.simple_right.execute(pctx, ectx, sctx);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: &mut ProofCtx<F>,
        ectx: &ExecutionCtx,
        sctx: &SetupCtx,
    ) {
        self.wcm.calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    _rom_path: Option<PathBuf>,
    _public_inputs_path: Option<PathBuf>,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp(None)
        .format_level(true)
        .format_target(false)
        .filter_level(log::LevelFilter::Trace)
        .init();
    let simple_witness = SimpleWitness::new();
    Ok(Box::new(simple_witness))
}
