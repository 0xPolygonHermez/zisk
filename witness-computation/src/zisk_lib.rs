use pil_std_lib::Std;
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use sm_rom::RomSM;
use std::{error::Error, path::PathBuf, sync::Arc};
use zisk_pil::*;

use p3_field::PrimeField;
use p3_goldilocks::Goldilocks;
use proofman::{WitnessLibrary, WitnessManager};
use proofman_common::{initialize_logger, ExecutionCtx, ProofCtx, SetupCtx, WitnessPilout};
use sm_arith::ArithSM;
use sm_binary::BinarySM;
use sm_main::MainSM;
use sm_mem::MemSM;

pub struct ZiskWitness<F: PrimeField> {
    pub public_inputs_path: PathBuf,
    pub rom_path: PathBuf,

    // Witness computation manager
    pub wcm: Option<Arc<WitnessManager<F>>>,

    // State machines
    pub main_sm: Option<Arc<MainSM<F>>>,
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

        Ok(ZiskWitness { public_inputs_path, rom_path, wcm: None, main_sm: None })
    }

    fn initialize(&mut self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        let wcm = WitnessManager::new(pctx, ectx, sctx.clone());
        let wcm = Arc::new(wcm);

        let std = Std::new(wcm.clone());

        let rom_sm = RomSM::new(wcm.clone(), sctx.clone());
        let mem_sm = MemSM::new(wcm.clone(), sctx.clone());
        let binary_sm = BinarySM::new(wcm.clone(), std.clone(), sctx.clone());
        let arith_sm = ArithSM::new(wcm.clone(), sctx.clone());

        let main_sm = MainSM::new(
            self.rom_path.clone(),
            wcm.clone(),
            sctx,
            rom_sm,
            mem_sm,
            binary_sm,
            arith_sm,
        );

        self.wcm = Some(wcm);
        self.main_sm = Some(main_sm);
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

        self.wcm.as_ref().unwrap().start_proof(pctx, ectx, sctx);
    }

    fn end_proof(&mut self) {
        self.wcm.as_ref().unwrap().end_proof();
    }
    fn execute(&self, pctx: Arc<ProofCtx<F>>, ectx: Arc<ExecutionCtx>, sctx: Arc<SetupCtx>) {
        timer_start_info!(EXECUTE);
        self.main_sm.as_ref().unwrap().execute(&self.public_inputs_path, pctx, ectx, sctx);
        timer_stop_and_log_info!(EXECUTE);
    }

    fn calculate_witness(
        &mut self,
        stage: u32,
        pctx: Arc<ProofCtx<F>>,
        ectx: Arc<ExecutionCtx>,
        sctx: Arc<SetupCtx>,
    ) {
        self.wcm.as_ref().unwrap().calculate_witness(stage, pctx, ectx, sctx);
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
