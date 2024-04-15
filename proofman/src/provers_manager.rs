use std::collections::HashMap;

use util::{timer_start, timer_stop_and_log};
use crate::AirInstanceCtx;
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
    fn get_challenges(&mut self) -> &mut [T];
    fn get_subproof_values(&mut self) -> &mut [T];
}

// PROVERS MANAGER
// ================================================================================================
pub struct ProversManager<T, PB> {
    prover_builder: PB,
    provers_map: HashMap<String, Box<dyn Prover<T>>>,
}

impl<T, PB> ProversManager<T, PB>
where
    T: Default + Clone,
    PB: ProverBuilder<T>,
{
    const MY_NAME: &'static str = "prvrsMan";

    pub fn new(prover_builder: PB) -> Self {
        debug!("{}: Initializing", Self::MY_NAME);

        Self { prover_builder, provers_map: HashMap::new() }
    }

    pub fn new_proof(&self) {
        todo!("{}: ==> NEW PROOF", Self::MY_NAME);
    }

    pub fn setup(&mut self /*&public_inputs, &self.options*/) {
        debug!("{}: ==> SETUP", Self::MY_NAME);
    }

    fn get_prover_id_from_air_instance(air_instance: &AirInstanceCtx<T>) -> String {
        format!("{}-{}-{}", air_instance.subproof_id, air_instance.air_id, air_instance.instance_id)
    }

    pub fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        // After computing the witness on stage 1, we assume we know the value of N for all air instances.
        // This allows us to construct each air instance prover depending on its features.

        if stage_id == 1 {
            // self.new_proof();

            timer_start!(BUILDING_PROVERS);
            debug!("{}: ==> CREATING PROVERS {}", Self::MY_NAME, stage_id);

            for subproof_ctx in proof_ctx.subproofs.iter() {
                for air_ctx in subproof_ctx.airs.iter() {
                    if true {
                        let prover_id = "0-0-0".to_string();
                        let name = "zkevm";
                        debug!("{}: ··· Creating prover '{}' id: {}", Self::MY_NAME, name, prover_id);

                        let prover = self.prover_builder.build();
                        self.provers_map.insert(prover_id, prover);
                    } else {
                        for air_instance in air_ctx.instances.iter() {
                            let prover_id = Self::get_prover_id_from_air_instance(&air_instance);
                            let name = proof_ctx.pilout.name(air_instance.subproof_id, air_instance.air_id);

                            debug!("{}: ··· Creating prover '{}' id: {}", Self::MY_NAME, name, prover_id);

                            let prover = self.prover_builder.build();
                            self.provers_map.insert(prover_id, prover);
                        }
                    }
                }
            }

            debug!("{}: <== CREATING PROVERS {}", Self::MY_NAME, stage_id);
            timer_stop_and_log!(BUILDING_PROVERS);
        }

        let num_stages = proof_ctx.pilout.num_stages();

        let status = if stage_id <= num_stages + 1 {
            // Commit phase
            self.commit_stage(stage_id, proof_ctx)
        } else {
            // Openings phase
            self.opening_stage(stage_id - num_stages - 1, proof_ctx)
        };

        // if status != ProverStatus::StagesCompleted {
        //     let challenge = self.compute_global_challenge(stage_id, proof_ctx);
        //     self.set_global_challenge(challenge, proof_ctx);
        // }

        status
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        trace!("{}: ==> COMMIT STAGE {}", Self::MY_NAME, stage_id);

        let mut status: Option<ProverStatus> = None;
        for (_, prover) in self.provers_map.iter_mut() {
            let _status = prover.commit_stage(stage_id, proof_ctx);
            if status.is_none() {
                status = Some(_status);
            }
        }

        trace!("{}: <== COMMIT STAGE {}", Self::MY_NAME, stage_id);

        status.unwrap()
    }

    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        trace!("{}: ==> OPENING STAGE {}", Self::MY_NAME, opening_id);

        let mut status: Option<ProverStatus> = None;
        for (_, prover) in self.provers_map.iter_mut() {
            let _status = prover.opening_stage(opening_id, proof_ctx);
            if status.is_none() {
                status = Some(_status);
            }
        }

        trace!("{}: <== OPENING STAGE {}", Self::MY_NAME, opening_id);

        status.unwrap()
    }

    pub fn verify_constraints(&self, stage_id: u32) -> bool {
        trace!("{}: ==> VERIFY CONSTRAINTS {}", Self::MY_NAME, stage_id);

        false
    }

    pub fn verify_global_constraints(&self) -> bool {
        trace!("{}: ==> VERIFY GLOBAL CONSTRAINTS", Self::MY_NAME);

        false
    }

    fn _compute_global_challenge(&mut self, _stage_id: u32, _proof_ctx: &ProofCtx<T>) -> T {
        // trace!("{}: ··· Compute global challlenge (stage {})", Self::MY_NAME, stage_id);

        // for subproof_ctx in proof_ctx.subproofs.iter() {
        //     for air_ctx in subproof_ctx.airs.iter() {
        //         for air_instance in air_ctx.instances.iter() {
        // let prover_id = Self::get_prover_id_from_air_instance(&air_instance);

        // let prover = self.provers_map.get_mut(&prover_id).unwrap();

        //let challenges = prover.get_challenges();
        // let prover = self.prover_builder.build();
        // self.provers_map.insert(prover_id, prover);
        // }
        //     }
        // }

        // unimplemented!("{}: ==> COMPUTE NEXT CHALLENGE {}", Self::MY_NAME, stage_id);
        T::default()
    }

    fn _set_global_challenge(&self, _challenge: T, _proof_ctx: &ProofCtx<T>) {
        // for (const airInstance of this.proofCtx.airInstances) {
        //     airInstance.ctx.challenges[stageId] = challenges;
        // }

        // unimplemented!("{}: ==> SET GLOBAL CHALLENGE", Self::MY_NAME);
    }
}
