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
pub struct ProversManager<T, PB> {
    prover_builder: PB,
    provers: Vec<Box<dyn Prover<T>>>,
}

impl<T, PB> ProversManager<T, PB>
where
    PB: ProverBuilder<T>,
{
    const MY_NAME: &'static str = "prvrsMan";

    pub fn new(prover_builder: PB) -> Self {
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

        // if status != ProverStatus::StagesCompleted {
        //     let challenge = self.compute_global_challenge(stage_id, proof_ctx);
        //     self.set_global_challenge(challenge, proof_ctx);
        // }

        status
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        trace!("{}: ==> COMMIT STAGE {}", Self::MY_NAME, stage_id);

        // for prover in self.provers.iter() {
        //     prover.compute_stage(stage_id, proof_ctx);
        // }
        let status = self.provers[0].commit_stage(stage_id, proof_ctx);
        trace!("{}: <== COMMIT STAGE {}", Self::MY_NAME, stage_id);

        status
    }

    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
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

    fn _compute_global_challenge(&self, stage_id: u32, proof_ctx: &ProofCtx<T>) -> T {
        trace!("{}: ··· Compute global challlenge (stage {})", Self::MY_NAME, stage_id);

        if stage_id == 1 {
            //let public_values;
            for subproof in proof_ctx.subproofs.iter() {
                // let challenges = Vec::new();

                for air in subproof.airs.iter() {
                    let air_instances = &proof_ctx.subproofs[air.subproof_id].airs[air.air_id].instances;

                    for (instance_id, _instance) in air_instances.iter().enumerate() {
                        trace!(
                            "{}: ··· Computing global challenge. Adding constTree. Subproof {} Air {} Instance {}",
                            Self::MY_NAME,
                            air.subproof_id,
                            air.air_id,
                            instance_id
                        );

                        // challenges.push(air.setup.const_root);

                        // if !publicValues {
                        //     publicValues = airInstance.ctx.publics;
                        // }
                    }
                }
            }
        }

        for subproof in proof_ctx.subproofs.iter() {
            // let challenges = Vec::new();

            for air in subproof.airs.iter() {
                let air_instances = &proof_ctx.subproofs[air.subproof_id].airs[air.air_id].instances;

                for (instance_id, _instance) in air_instances.iter().enumerate() {
                    trace!(
                        "{}: ··· Computing global challenge. Adding subproof {} Air {} Instance {} value",
                        Self::MY_NAME,
                        air.subproof_id,
                        air.air_id,
                        instance_id
                    );

                    // let value = air_instances[instance_id].get_value();
                    // challenges.push(value);

                    // if (options.vadcop) {
                    //     if(challenges.length > 0) {
                    //         const challenge = await hashBTree(challenges);
                    //         this.proofCtx.addChallengeToTranscript(challenge);
                    //     }
                    // } else {
                    //     for (let k = 0; k < challenges.length; k++) {
                    //         this.proofCtx.addChallengeToTranscript(challenges[k]);
                    //     }
                    // }
                }
            }
        }

        unimplemented!("{}: ==> COMPUTE NEXT CHALLENGE {}", Self::MY_NAME, stage_id);
    }

    fn _set_global_challenge(&self, _challenge: T, _proof_ctx: &ProofCtx<T>) {
        // for (const airInstance of this.proofCtx.airInstances) {
        //     airInstance.ctx.challenges[stageId] = challenges;
        // }

        unimplemented!("{}: ==> SET GLOBAL CHALLENGE", Self::MY_NAME);
    }
}
