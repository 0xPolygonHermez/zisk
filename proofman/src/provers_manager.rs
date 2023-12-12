use log::debug;
use crate::prover::Prover;
use crate::proof_manager::ProverStatus;

use log::info;

// PROVERS MANAGER
// ================================================================================================
pub struct ProversManager {
    _prover: Box<dyn Prover>,
}

impl ProversManager {
    const MY_NAME: &'static str = "proversm";

    pub fn new(prover: Box<dyn Prover>) -> Self {
        debug!("{}> Initializing...", Self::MY_NAME);

        Self {
            _prover: prover
        }
    }

    pub fn setup(&mut self, /*&public_inputs, &self.options*/) {
        info!("{}> ==> SETUP", Self::MY_NAME);
    }

    pub fn compute_stage(&mut self, stage_id: usize, /*&public_inputs, &self.options*/) -> ProverStatus {
        info!("{}> ==> COMPUTE STAGE {}", Self::MY_NAME, stage_id);

        ProverStatus::OpeningsCompleted
    }

    pub fn verify_constraints(&self, stage_id: usize) -> bool {
        info!("{}> ==> VERIFY CONSTRAINTS {}", Self::MY_NAME, stage_id);

        false
    }

    pub fn verify_global_constraints(&self) -> bool {
        info!("{}> ==> VERIFY GLOBAL CONSTRAINTS", Self::MY_NAME);

        false
    }
}