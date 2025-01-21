use std::sync::{Arc, RwLock};
use std::path::PathBuf;

use proofman_common::{ProofCtx, SetupCtx};
use proofman_util::{timer_start_info, timer_stop_and_log_info};
use crate::WitnessComponent;

pub struct WitnessManager<F> {
    components: RwLock<Vec<Arc<dyn WitnessComponent<F>>>>,
    pctx: Arc<ProofCtx<F>>,
    sctx: Arc<SetupCtx>,
    rom_path: Option<PathBuf>,
    public_inputs_path: Option<PathBuf>,
}

impl<F> WitnessManager<F> {
    const MY_NAME: &'static str = "WCMnager";

    pub fn new(
        pctx: Arc<ProofCtx<F>>,
        sctx: Arc<SetupCtx>,
        rom_path: Option<PathBuf>,
        public_inputs_path: Option<PathBuf>,
    ) -> Self {
        WitnessManager { components: RwLock::new(Vec::new()), pctx, sctx, rom_path, public_inputs_path }
    }

    pub fn register_component(&self, component: Arc<dyn WitnessComponent<F>>) {
        self.components.write().unwrap().push(component);
    }

    pub fn start_proof(&self) {
        timer_start_info!(START_PROOF);
        for component in self.components.read().unwrap().iter() {
            component.start_proof(self.pctx.clone(), self.sctx.clone());
        }
        timer_stop_and_log_info!(START_PROOF);
    }

    pub fn execute(&self) {
        timer_start_info!(EXECUTE);
        for component in self.components.read().unwrap().iter() {
            component.execute(self.pctx.clone());
        }
        timer_stop_and_log_info!(EXECUTE);
    }

    pub fn debug(&self) {
        for component in self.components.read().unwrap().iter() {
            component.debug(self.pctx.clone());
        }
    }

    pub fn end_proof(&self) {
        for component in self.components.read().unwrap().iter() {
            component.end_proof();
        }
    }

    pub fn calculate_witness(&self, stage: u32) {
        log::info!(
            "{}: Calculating witness for stage {} / {}",
            Self::MY_NAME,
            stage,
            self.pctx.global_info.n_challenges.len()
        );

        timer_start_info!(CALCULATING_WITNESS);

        // Call one time all unused components
        for component in self.components.read().unwrap().iter() {
            component.calculate_witness(stage, self.pctx.clone(), self.sctx.clone());
        }

        timer_stop_and_log_info!(CALCULATING_WITNESS);
    }

    pub fn get_pctx(&self) -> Arc<ProofCtx<F>> {
        self.pctx.clone()
    }

    pub fn get_sctx(&self) -> Arc<SetupCtx> {
        self.sctx.clone()
    }

    pub fn get_rom_path(&self) -> Option<PathBuf> {
        self.rom_path.clone()
    }

    pub fn get_public_inputs_path(&self) -> Option<PathBuf> {
        self.public_inputs_path.clone()
    }
}
