use util::{timer_start, timer_stop_and_log};
use crate::proof_manager::ProverStatus;
use crate::proof_ctx::ProofCtx;

use log::{debug, trace};

pub trait ProverBuilder<T> {
    fn build(&mut self) -> Box<dyn Prover<T>>;
}

pub trait Prover<T> {
    fn build(&mut self);
    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus;
    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus;
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

    pub fn new_proof(&self) {
        todo!("{}: ==> NEW PROOF", Self::MY_NAME);
    }

    pub fn setup(&mut self /*&public_inputs, &self.options*/) {
        debug!("{}: ==> SETUP", Self::MY_NAME);
    }

    pub fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        // After computing the witness on stage 1, we assume we know the value of N for all air instances.
        // This allows us to construct each air instance prover depending on its features.
        if stage_id == 1 {
            // TODO! Uncomment when implemented
            // self.new_proof();

            timer_start!(BUILDING_PROVERS);
            debug!("{}: ==> CREATING PROVERS {}", Self::MY_NAME, stage_id);

            // TODO! When VADCOPS we will iterate and select the prover for each air instance.
            let prover = self.prover_builder.build();
            self.provers.push(prover);

            debug!("{}: <== CREATING PROVERS {}", Self::MY_NAME, stage_id);
            timer_stop_and_log!(BUILDING_PROVERS);
        }

        // TODO! Uncomment this when pilout done!!!!
        // let num_stages = proof_ctx.pilout.get_num_stages();
        let num_stages = 4;

        let status = if stage_id <= num_stages {
            // Commit phase
            self.commit_stage(stage_id, proof_ctx)
        } else {
            // Openings phase
            self.opening_stage(stage_id - num_stages, proof_ctx)
        };

        status
    }

    pub fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        trace!("{}: ==> COMMIT STAGE {}", Self::MY_NAME, stage_id);

        // for prover in self.provers.iter() {
        //     prover.compute_stage(stage_id, proof_ctx);
        // }
        let status = self.provers[0].commit_stage(stage_id, proof_ctx);
        trace!("{}: <== COMMIT STAGE {}", Self::MY_NAME, stage_id);

        status
    }

    pub fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        trace!("{}: ==> OPENING STAGE {}", Self::MY_NAME, opening_id);

        // for prover in self.provers.iter() {
        //     prover.opening_stage(stage_id, proof_ctx);
        // }
        // let stage = self.provers[0].opening_stage(stage_id, proof_ctx);

        // state
        let status = self.provers[0].opening_stage(opening_id, proof_ctx);
        trace!("{}: <== OPENING STAGE {}", Self::MY_NAME, opening_id);

        status
    }

    pub fn verify_constraints(&self, stage_id: u32) -> bool {
        trace!("{}: ==> VERIFY CONSTRAINTS {}", Self::MY_NAME, stage_id);

        false
    }

    pub fn verify_global_constraints(&self) -> bool {
        trace!("{}: ==> VERIFY GLOBAL CONSTRAINTS", Self::MY_NAME);

        false
    }
}
