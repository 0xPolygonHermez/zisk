use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Lookup0, Lookup1, Lookup2_12, Lookup2_13, Lookup2_15, Lookup3, Pilout};

pub struct LookupWitness<F: PrimeField> {
    pub wcm: WitnessManager<F>,
    pub lookup0: Arc<Lookup0<F>>,
    pub lookup1: Arc<Lookup1<F>>,
    pub lookup2_12: Arc<Lookup2_12<F>>,
    pub lookup2_13: Arc<Lookup2_13<F>>,
    pub lookup2_15: Arc<Lookup2_15<F>>,
    pub lookup3: Arc<Lookup3<F>>,
    pub std_lib: Arc<Std<F>>,
}

impl<F: PrimeField> Default for LookupWitness<F>
where
    Standard: Distribution<F>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<F: PrimeField> LookupWitness<F>
where
    Standard: Distribution<F>,
{
    pub fn new() -> Self {
        let mut wcm = WitnessManager::new();

        let std_lib = Std::new(&mut wcm, None);
        let lookup0 = Lookup0::new(&mut wcm);
        let lookup1 = Lookup1::new(&mut wcm);
        let lookup2_12 = Lookup2_12::new(&mut wcm);
        let lookup2_13 = Lookup2_13::new(&mut wcm);
        let lookup2_15 = Lookup2_15::new(&mut wcm);
        let lookup3 = Lookup3::new(&mut wcm);

        LookupWitness {
            wcm,
            lookup0,
            lookup1,
            lookup2_12,
            lookup2_13,
            lookup2_15,
            lookup3,
            std_lib,
        }
    }
}

impl<F: PrimeField> WitnessLibrary<F> for LookupWitness<F>
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
        self.lookup0.execute(pctx, ectx, sctx);
        self.lookup1.execute(pctx, ectx, sctx);
        self.lookup2_12.execute(pctx, ectx, sctx);
        self.lookup2_13.execute(pctx, ectx, sctx);
        self.lookup2_15.execute(pctx, ectx, sctx);
        self.lookup3.execute(pctx, ectx, sctx);
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
    let lookup_witness = LookupWitness::new();
    Ok(Box::new(lookup_witness))
}
