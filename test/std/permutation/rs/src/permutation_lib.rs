use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Permutation1, Permutation1_1, Permutation1_2, Permutation2, Pilout};

pub struct PermutationWitness<F: PrimeField> {
    pub wcm: WitnessManager<F>,
    pub permutation1: Arc<Permutation1<F>>,
    pub permutation1_1: Arc<Permutation1_1<F>>,
    pub permutation1_2: Arc<Permutation1_2<F>>,
    pub permutation2: Arc<Permutation2<F>>,
    pub std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> Default for PermutationWitness<F>
where
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> PermutationWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        let mut wcm = WitnessManager::new();

        let std_lib = Std::new(&mut wcm, None);
        let permutation1 = Permutation1::new(&mut wcm);
        let permutation1_1 = Permutation1_1::new(&mut wcm);
        let permutation1_2 = Permutation1_2::new(&mut wcm);
        let permutation2 = Permutation2::new(&mut wcm);

        PermutationWitness {
            wcm,
            permutation1,
            permutation1_1,
            permutation1_2,
            permutation2,
            std_lib,
        }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for PermutationWitness<F>
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
        self.permutation1.execute(pctx, ectx, sctx);
        self.permutation1_1.execute(pctx, ectx, sctx);
        self.permutation1_2.execute(pctx, ectx, sctx);
        self.permutation2.execute(pctx, ectx, sctx);
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
    let permutation_witness = PermutationWitness::new();
    Ok(Box::new(permutation_witness))
}
