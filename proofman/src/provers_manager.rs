use log::debug;
use util::{timer_start, timer_stop_and_log};
use std::time::Instant;
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
        // After computing the witness on stage 1, we assume we know the value of N for all air instances.
        // This allows us to construct each air instance prover depending on its features.
        if stage_id == 1 {
            timer_start!(BUILDING_PROVERS);
            info!("{}: ==> CREATING PROVERS {}", Self::MY_NAME, stage_id);

            // TODO! When VADCOPS we will iterate and select the prover for each air instance.
            let prover = self.prover_builder.build();
            self.provers.push(prover);

            info!("{}: <== CREATING PROVERS {}", Self::MY_NAME, stage_id);
            timer_stop_and_log!(BUILDING_PROVERS);
        }

        info!("{}: ==> COMPUTE STAGE {}", Self::MY_NAME, stage_id);

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
