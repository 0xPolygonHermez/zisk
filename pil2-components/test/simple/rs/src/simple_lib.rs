use std::{error::Error, path::PathBuf, sync::Arc};

use pil_std_lib::Std;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{initialize_logger, ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use rand::{distributions::Standard, prelude::Distribution};

use crate::{Pilout, SimpleLeft, SimpleRight};

pub struct SimpleWitness<F: PrimeField> {
    pub wcm: Option<Arc<WitnessManager<F>>>,
    pub simple_left: Option<Arc<SimpleLeft<F>>>,
    pub simple_right: Option<Arc<SimpleRight<F>>>,
    pub std_lib: Option<Arc<Std<F>>>,
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
        Self { wcm: None, simple_left: None, simple_right: None, std_lib: None }
    }

    pub fn initialize(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = Arc::new(WitnessManager::new(pctx, ectx, sctx));

        let std_lib = Std::new(wcm.clone());
        let simple_left = SimpleLeft::new(wcm.clone());
        let simple_right = SimpleRight::new(wcm.clone());

        self.wcm = Some(wcm);
        self.std_lib = Some(std_lib);
        self.simple_left = Some(simple_left);
        self.simple_right = Some(simple_right);
    }
}

impl<F: PrimeField> WitnessLibrary<F> for SimpleWitness<F>
where
    Standard: Distribution<F>,
{
    fn start_proof(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.initialize(pctx.clone(), ectx.clone(), sctx.clone());

        self.wcm.as_ref().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }

    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        // Execute those components that need to be executed
        self.simple_left.as_ref().unwrap().execute(pctx.clone(), ectx.clone(), sctx.clone());
        self.simple_right.as_ref().unwrap().execute(pctx, ectx, sctx);
    }

    fn calculate_witness(&mut self, stage: u32, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        self.wcm.as_ref().unwrap().calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    ectx: Arc<ExecutionCtx>,
    _: Option<PathBuf>,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    initialize_logger(ectx.verbose_mode);

    let simple_witness = SimpleWitness::new();
    Ok(Box::new(simple_witness))
}
