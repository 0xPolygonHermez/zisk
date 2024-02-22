use log::debug;
use crate::proof_manager::ProverStatus;
use crate::proof_ctx::ProofCtx;

use log::info;

pub trait ProverBuilder<T> {
    fn build(&mut self) -> Box<dyn Prover<T>>;
}

pub trait Prover<T> {
    fn build(&mut self);
    fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>);
}

// PROVERS MANAGER
// ================================================================================================
pub struct ProversManager<T> {
    prover_builder: Box<dyn ProverBuilder<T>>,
    provers: Vec<Box<dyn Prover<T>>>,
}

impl<T> ProversManager<T> {
    const MY_NAME: &'static str = "prvrsMan";

    pub fn new(prover_builder: Box<dyn ProverBuilder<T>>) -> Self {
        debug!("{}: Initializing", Self::MY_NAME);

        Self { prover_builder, provers: Vec::new() }
    }

    pub fn setup(&mut self /*&public_inputs, &self.options*/) {
        info!("{}: ==> SETUP", Self::MY_NAME);
    }

    pub fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        info!("{}: ==> COMPUTE STAGE {}", Self::MY_NAME, stage_id);

        let prover = self.prover_builder.build();
        self.provers.push(prover);
        self.provers[0].compute_stage(stage_id, proof_ctx);

        info!("{}: <== COMPUTE STAGE {}", Self::MY_NAME, stage_id);

        ProverStatus::StagesCompleted
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
