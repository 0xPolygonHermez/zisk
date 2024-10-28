use proofman_util::{timer_start_info, timer_stop_and_log_info};
use std::{cell::OnceCell, error::Error, path::PathBuf, sync::Arc};
use zisk_pil::*;

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{initialize_logger, ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};

use crate::ZiskExecutor;

pub struct ZiskWitness<F: PrimeField> {
    /// Public inputs path
    pub public_inputs_path: PathBuf,

    /// ROM path
    pub rom_path: PathBuf,

    /// Witness computation manager
    pub wcm: OnceCell<Arc<WitnessManager<F>>>,

    /// Executor
    pub executor: OnceCell<ZiskExecutor<F>>,
}

impl<F: PrimeField> ZiskWitness<F> {
    pub fn new(rom_path: PathBuf, public_inputs_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        // Check rom_path path exists
        if !rom_path.exists() {
            return Err(format!("ROM file not found at path: {:?}", rom_path).into());
        }

        // Check public_inputs_path is a folder
        if !public_inputs_path.exists() {
            return Err(
                format!("Public inputs file not found at path: {:?}", public_inputs_path).into()
            );
        }

        Ok(ZiskWitness {
            public_inputs_path,
            rom_path,
            wcm: OnceCell::new(),
            executor: OnceCell::new(),
        })
    }

    fn initialize(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = WitnessManager::new(pctx, ectx, sctx.clone());
        let wcm = Arc::new(wcm);

        self.wcm.set(wcm.clone()).ok();
        self.executor.set(ZiskExecutor::new(wcm, self.rom_path.clone())).ok();
    }
}

impl<F: PrimeField> WitnessLibrary<F> for ZiskWitness<F> {
    fn start_proof(
        &mut self,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.initialize(pctx.clone(), ectx.clone(), sctx.clone());

        self.wcm.get().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.get().unwrap().end_proof();
    }
    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        timer_start_info!(EXECUTE);
        self.executor.get().unwrap().execute(self.public_inputs_path.clone(), pctx, ectx, sctx);
        timer_stop_and_log_info!(EXECUTE);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.wcm.get().unwrap().calculate_witness(stage, pctx, ectx, sctx);
    }

    fn pilout(&self) -> WitnessPilout {
        Pilout::pilout()
    }
}

#[no_mangle]
pub extern "Rust" fn init_library(
    ectx: Arc<ExecutionCtx>,
    public_inputs_path: Option<PathBuf>,
) -> Result<Box<dyn WitnessLibrary<Goldilocks>>, Box<dyn Error>> {
    let rom_path = ectx.rom_path.clone().ok_or("ROM path is required")?;
    let public_inputs = public_inputs_path.ok_or("Public inputs path is required")?;

    initialize_logger(ectx.verbose_mode);

    let zisk_witness = ZiskWitness::new(rom_path, public_inputs)?;
    Ok(Box::new(zisk_witness))
}
