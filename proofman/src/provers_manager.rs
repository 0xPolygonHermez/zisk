use log::debug;
use crate::proof_manager::ProverStatus;
use crate::proof_ctx::ProofCtx;

use log::info;

pub trait Prover<T> {
    fn compute_stage(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}

// PROVERS MANAGER
// ================================================================================================
pub struct ProversManager<T> {
    prover: Box<dyn Prover<T>>,
}

impl<T> ProversManager<T> {
    const MY_NAME: &'static str = "prvrsMan";

    pub fn new(prover: Box<dyn Prover<T>>) -> Self {
        debug!("{}: Initializing", Self::MY_NAME);

        Self { prover }
    }

    pub fn setup(&mut self /*&public_inputs, &self.options*/) {
        info!("{}: ==> SETUP", Self::MY_NAME);
    }

    pub fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        info!("{}: ==> COMPUTE STAGE {}", Self::MY_NAME, stage_id);

        self.prover.compute_stage(stage_id, proof_ctx);

        info!("{}: <== COMPUTE STAGE {}", Self::MY_NAME, stage_id);

        ProverStatus::OpeningsCompleted
    }

    pub fn verify_constraints(&self, stage_id: u32) -> bool {
        info!("{}: ==> VERIFY CONSTRAINTS {}", Self::MY_NAME, stage_id);

        false
    }

    pub fn verify_global_constraints(&self) -> bool {
        info!("{}: ==> VERIFY GLOBAL CONSTRAINTS", Self::MY_NAME);

        false
    }
}
