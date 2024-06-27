use std::collections::HashMap;

use goldilocks::AbstractField;
use pilout::pilout::AggregationType;
use util::{timer_start, timer_stop_and_log};
use crate::AirInstanceCtx;
use crate::proof_manager::ProverStatus;
use crate::proof_ctx::ProofCtx;

use crate::hash_btree::hash_btree_256;

use log::{debug, trace};

pub trait ProverBuilder<T> {
    fn build(&mut self, air_instance_ctx: &AirInstanceCtx<T>) -> Box<dyn Prover<T>>;
    fn create_buffer(&mut self) -> Vec<u8>;
}

pub trait Prover<T> {
    fn build(&mut self, air_instance_ctx: &AirInstanceCtx<T>);
    fn num_stages(&self) -> u32;
    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus;
    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus;

    // Returns a slice representing the root of a Merkle tree with a size of 256 bits.
    // This root can be inserted into a transcript and used to generate a new challenge.
    // Due to implementation reasons, we return a slice of 4 elements, each of 64 bits.
    fn get_commit_stage_root_challenge_256(&self, stage_id: u32) -> Option<[u64; 4]>;
    fn get_opening_stage_root_challenge_256(&self, opening_id: u32) -> Option<[u64; 4]>;
    fn add_root_challenge_256_to_transcript(&mut self, root_challenge: [u64; 4]);

    fn get_subproof_values(&self) -> Vec<T>;
}

// PROVERS MANAGER
// ================================================================================================
pub struct ProversManager<T, PB> {
    pub prover_builders: HashMap<String, PB>,
    provers_map: HashMap<String, Box<dyn Prover<T>>>,
    num_stages: Option<u32>,
    // TODO! This flag is used only while developing vadcops. After that it must be removed.
    // TODO! It allow us to inidicate that we are using a BIG trace matrix instead of a fully enhanced vadcops as it is used in the current zkEVM implementation.
    // TODO! This flag must be removed after the implementation of vadcops.
    dev_use_feature: bool,
}

impl<T, PB> ProversManager<T, PB>
where
    T: Default + Copy + Clone + AbstractField,
    PB: ProverBuilder<T>,
{
    const MY_NAME: &'static str = "prvrsMan";

    pub fn new(prover_builders: HashMap<String, PB>, dev_use_feature: bool) -> Self {
        debug!("{}: Initializing", Self::MY_NAME);

        Self { prover_builders, provers_map: HashMap::new(), dev_use_feature, num_stages: None }
    }

    pub fn new_proof(&self) {
        todo!("{}: ==> NEW PROOF", Self::MY_NAME);
    }

    pub fn init_provers(&mut self, proof_ctx: &ProofCtx<T>) {
        // self.new_proof();
        timer_start!(SETUP_PROVERS);
        debug!("{}: ==> CREATING PROVERS", Self::MY_NAME);

        for subproof_ctx in proof_ctx.subproofs.iter() {
            for air_ctx in subproof_ctx.airs.iter() {
                for air_instance in air_ctx.instances.iter() {
                    let prover_id = Self::get_prover_id_from_air_instance(&air_instance);
                    let name = if self.dev_use_feature {
                        "zkevm"
                    } else {
                        proof_ctx.pilout.name(air_instance.subproof_id, air_instance.air_id)
                    };

                    debug!("{}: ··· Creating prover '{}' id: {}", Self::MY_NAME, name, prover_id);

                    let prover = self
                        .prover_builders
                        .get_mut(name)
                        .unwrap_or_else(|| panic!("{}: Prover '{}' not found", Self::MY_NAME, name))
                        .build(air_instance);

                    if subproof_ctx.subproof_id == 0 && air_ctx.air_id == 0 {
                        self.num_stages = Some(prover.num_stages());
                    }
                    self.provers_map.insert(prover_id, prover);
                }
            }
        }

        debug!("{}: <== CREATING PROVERS", Self::MY_NAME);
        timer_stop_and_log!(SETUP_PROVERS);
    }

    // This function is used to get the number of stages of the prover.
    pub fn num_stages(&self) -> Option<u32> {
        self.num_stages
    }

    fn get_prover_id_from_air_instance(air_instance: &AirInstanceCtx<T>) -> String {
        format!("{}-{}-{}", air_instance.subproof_id, air_instance.air_id, air_instance.instance_id)
    }

    pub fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        let status = if stage_id <= self.num_stages.unwrap() + 1 {
            // Commit phase
            let status = self.commit_stage(stage_id, proof_ctx);

            if status != ProverStatus::StagesCompleted {
                let challenge = self.compute_commit_stage_global_challenge(stage_id, proof_ctx);
                if let Some(ch) = challenge {
                    self.set_global_challenge(ch, stage_id, proof_ctx);
                }
            }
            status
        } else {
            let opening_id = stage_id - self.num_stages.unwrap() - 1;
            // Openings phase
            let status = self.opening_stage(opening_id, proof_ctx);

            if status != ProverStatus::StagesCompleted {
                let challenge = self.compute_opening_stage_global_challenge(opening_id, proof_ctx);
                if let Some(ch) = challenge {
                    self.set_global_challenge(ch, stage_id, proof_ctx);
                }
            }
            status
        };

        // Compute subproof values
        if stage_id == self.num_stages.unwrap() {
            self.update_subproof_values(proof_ctx);
            self.compute_subproof_values(proof_ctx);
        }

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

    fn update_subproof_values(&mut self, proof_ctx: &mut ProofCtx<T>) {
        trace!("{}: ==> UPDATE SUBPROOF VALUES", Self::MY_NAME);

        for subproof_ctx in proof_ctx.subproofs.iter_mut() {
            for air_ctx in subproof_ctx.airs.iter_mut() {
                for air_instance in air_ctx.instances.iter_mut() {
                    let prover_id = Self::get_prover_id_from_air_instance(&air_instance);

                    let prover = self.provers_map.get(&prover_id).unwrap();

                    air_instance.subproof_values = prover.get_subproof_values();
                }
            }
        }

        trace!("{}: <== UPDATE SUBPROOF VALUES", Self::MY_NAME);
    }

    pub fn verify_constraints(&self, stage_id: u32) -> bool {
        trace!("{}: ==> VERIFY CONSTRAINTS {}", Self::MY_NAME, stage_id);

        false
    }

    pub fn verify_global_constraints(&self) -> bool {
        trace!("{}: ==> VERIFY GLOBAL CONSTRAINTS", Self::MY_NAME);

        false
    }

    fn compute_commit_stage_global_challenge(&mut self, stage_id: u32, proof_ctx: &ProofCtx<T>) -> Option<[u64; 4]> {
        trace!("{}: ··· Compute commit stage global challlenge (stage {})", Self::MY_NAME, stage_id);

        let mut challenges = Vec::new();

        for subproof_ctx in proof_ctx.subproofs.iter() {
            for air_ctx in subproof_ctx.airs.iter() {
                for air_instance in air_ctx.instances.iter() {
                    let prover_id = Self::get_prover_id_from_air_instance(&air_instance);

                    let prover = self.provers_map.get(&prover_id).unwrap();

                    let challenge = prover.get_commit_stage_root_challenge_256(stage_id);

                    if let Some(ch) = challenge {
                        challenges.push(ch);
                    }
                }
            }
        }

        if challenges.is_empty() {
            return None;
        }

        let global_challenge = hash_btree_256(&mut challenges)
            .unwrap_or_else(|_| panic!("{}: Error computing global challenge", Self::MY_NAME));

        Some(global_challenge)
    }

    fn compute_opening_stage_global_challenge(&mut self, opening_id: u32, proof_ctx: &ProofCtx<T>) -> Option<[u64; 4]> {
        trace!("{}: ··· Compute opening stage global challlenge (stage {})", Self::MY_NAME, opening_id);

        let mut challenges = Vec::new();

        for subproof_ctx in proof_ctx.subproofs.iter() {
            for air_ctx in subproof_ctx.airs.iter() {
                for air_instance in air_ctx.instances.iter() {
                    let prover_id = Self::get_prover_id_from_air_instance(&air_instance);

                    let prover = self.provers_map.get(&prover_id).unwrap();

                    let challenge = prover.get_opening_stage_root_challenge_256(opening_id);

                    if let Some(ch) = challenge {
                        challenges.push(ch);
                    }
                }
            }
        }

        if challenges.is_empty() {
            return None;
        }

        let global_challenge = hash_btree_256(&mut challenges)
            .unwrap_or_else(|_| panic!("{}: Error computing global challenge", Self::MY_NAME));

        Some(global_challenge)
    }

    fn set_global_challenge(&mut self, global_challenge: [u64; 4], stage_id: u32, proof_ctx: &ProofCtx<T>) {
        trace!(
            "{}: ··· Set global challlenge (stage {}): [{}, {}, {}, {}]",
            Self::MY_NAME,
            stage_id,
            global_challenge[0],
            global_challenge[1],
            global_challenge[2],
            global_challenge[3]
        );

        for subproof_ctx in proof_ctx.subproofs.iter() {
            for air_ctx in subproof_ctx.airs.iter() {
                for air_instance in air_ctx.instances.iter() {
                    let prover_id = Self::get_prover_id_from_air_instance(&air_instance);

                    let prover = self.provers_map.get_mut(&prover_id).unwrap();

                    prover.add_root_challenge_256_to_transcript(global_challenge);
                }
            }
        }
    }

    fn compute_subproof_values(&self, proof_ctx: &mut ProofCtx<T>) {
        for subproof_ctx in proof_ctx.subproofs.iter() {
            let subair_values = &proof_ctx.pilout.subproofs[subproof_ctx.subproof_id as usize].subproofvalues;
            println!("subair_values: {:?}", subair_values);
            for j in 0..subair_values.len() {
                let _agg_type = AggregationType::try_from(subair_values[j].agg_type)
                    .unwrap_or_else(|_| panic!("{}: Invalid aggregation type", Self::MY_NAME));

                for air_ctx in subproof_ctx.airs.iter() {
                    println!("air_ctx.instances: {:?}", air_ctx.instances.len());
                    for air_instance in air_ctx.instances.iter() {
                        println!("subproof_value: {:?}", air_instance.subproof_values);
                        // let subproof_value = air_instance.subproof_values[j];
                        // proof_ctx.subproof_values[subproof_ctx.subproof_id][j] = match agg_type {
                        //     AggregationType::Sum => {
                        //         // subproof_value + proof_ctx.subproof_values[subproof_ctx.subproof_id][j]
                        //         T::one()
                        //     }
                        //     AggregationType::Prod => {
                        //         // subproof_value * proof_ctx.subproof_values[subproof_ctx.subproof_id][j]
                        //         T::one()
                        //     }
                        // };
                    }
                }
            }
        }
    }
}
