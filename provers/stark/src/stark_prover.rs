use std::path::PathBuf;

use transcript::FFITranscript;
use core::slice;
use std::any::type_name;

use common::{Prover, ProverStatus};
use log::{debug, trace};
use util::{timer_start, timer_stop_and_log};
use starks_lib_c::*;
use common::{AirInstanceCtx, ProofCtx};
use crate::stark_info::{OpType, StarkInfo};
use crate::stark_prover_settings::StarkProverSettings;
use crate::GlobalInfo;
use p3_goldilocks::Goldilocks;
use p3_field::AbstractField;

use std::os::raw::c_void;

#[allow(dead_code)]
pub struct StarkProver<T: AbstractField> {
    initialized: bool,
    config: StarkProverSettings,
    p_chelpers: *mut c_void,
    p_steps: *mut c_void,
    pub p_stark: Option<*mut c_void>,
    p_params: Option<*mut c_void>,
    p_proof: Option<*mut c_void>,
    p_fri_pol: Option<*mut c_void>,
    stark_info: Option<StarkInfo>,
    p_starkinfo: *mut c_void,
    evals: Vec<T>,
    pub subproof_values: Vec<T>,
    n_field_elements: usize,
    merkle_tree_arity: Option<u64>,
    merkle_tree_custom: Option<bool>,

    // Pointers to the bool vector values inside the C++ code
    p_publics_calculated: *mut c_void,
    p_const_calculated: *mut c_void,
    p_subproof_values_calculated: *mut c_void,
    p_challenges_calculated: *mut c_void,
    p_witnesses_calculated: *mut c_void,
}

impl<T: AbstractField> StarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(proving_key_path: &PathBuf, global_info: &GlobalInfo, air_group_id: usize, air_id: usize) -> Self {
        let air_setup_folder = proving_key_path.join(global_info.get_air_setup_path(air_group_id, air_id));
        trace!("{}: ··· Setup AIR folder: {:?}", Self::MY_NAME, air_setup_folder);

        // Check path exists and is a folder
        if !air_setup_folder.exists() {
            panic!("Setup AIR folder not found at path: {:?}", air_setup_folder);
        }
        if !air_setup_folder.is_dir() {
            panic!("Setup AIR path is not a folder: {:?}", air_setup_folder);
        }

        let base_filename_path =
            air_setup_folder.join(global_info.get_air_name(air_group_id, air_id)).display().to_string();

        let stark_info_path = base_filename_path.clone() + ".starkinfo.json";
        let chelpers_path = base_filename_path.clone() + ".bin";

        let p_starkinfo = stark_info_new_c(&stark_info_path);
        let p_chelpers = chelpers_new_c(&chelpers_path);

        let config = StarkProverSettings {
            current_path: air_setup_folder.to_str().unwrap().to_string(),
            const_pols_filename: base_filename_path.clone() + ".const",
            map_const_pols_file: false,
            const_tree_filename: base_filename_path.clone() + ".consttree",
            stark_info_filename: stark_info_path,
            verkey_filename: base_filename_path.clone() + ".verkey.json",
            chelpers_filename: chelpers_path,
        };

        let p_steps = generic_steps_new_c();

        Self {
            initialized: true,
            config,
            p_chelpers,
            p_steps,
            p_stark: None,
            p_params: None,
            p_proof: None,
            p_fri_pol: None,
            stark_info: None,
            p_starkinfo,
            evals: Vec::new(),
            subproof_values: Vec::new(),
            n_field_elements: 0,
            merkle_tree_arity: None,
            merkle_tree_custom: None,
            p_publics_calculated: std::ptr::null_mut(),
            p_const_calculated: std::ptr::null_mut(),
            p_subproof_values_calculated: std::ptr::null_mut(),
            p_challenges_calculated: std::ptr::null_mut(),
            p_witnesses_calculated: std::ptr::null_mut(),
        }
    }

    pub fn get_stark_info(&self) -> *mut c_void {
        get_stark_info_c(self.p_stark.unwrap())
    }
}

impl<F: AbstractField> Prover<F> for StarkProver<F> {
    fn build(&mut self, air_instance_ctx: &mut AirInstanceCtx) {
        timer_start!(ESTARK_PROVER_NEW);

        let stark_info_json = std::fs::read_to_string(&self.config.stark_info_filename)
            .expect(format!("Failed to read file {}", &self.config.stark_info_filename).as_str());

        let ptr = air_instance_ctx.get_buffer_ptr() as *mut c_void;

        self.stark_info = Some(StarkInfo::from_json(&stark_info_json));

        let p_stark = starks_new_default_c(
            self.config.const_pols_filename.as_str(),
            self.config.map_const_pols_file,
            self.config.const_tree_filename.as_str(),
            self.p_starkinfo,
            self.p_chelpers,
            ptr,
        );

        self.p_stark = Some(p_stark);

        let stark_info = self.stark_info.as_ref().unwrap();

        //This is not necessary right now
        self.p_publics_calculated = get_vector_pointer_c(p_stark, "publicsCalculated");
        self.p_const_calculated = get_vector_pointer_c(p_stark, "constsCalculated");
        self.p_subproof_values_calculated = get_vector_pointer_c(p_stark, "subProofValuesCalculated");
        self.p_challenges_calculated = get_vector_pointer_c(p_stark, "challengesCalculated");
        self.p_witnesses_calculated = get_vector_pointer_c(p_stark, "witnessCalculated");

        if stark_info.stark_struct.verification_hash_type == "BN128" {
            self.n_field_elements = 1;
            self.merkle_tree_arity = Some(stark_info.stark_struct.merkle_tree_arity);
            self.merkle_tree_custom = Some(stark_info.stark_struct.merkle_tree_custom);
        } else {
            self.n_field_elements = Self::HASH_SIZE;
            self.merkle_tree_arity = Some(2);
            self.merkle_tree_custom = Some(true);
        }

        clean_symbols_calculated_c(p_stark);

        self.initialized = true;

        timer_stop_and_log!(ESTARK_PROVER_NEW);
    }

    fn num_stages(&self) -> u32 {
        self.stark_info.as_ref().unwrap().n_stages
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) -> ProverStatus {
        let p_stark: *mut std::ffi::c_void = self.p_stark.unwrap();
        let stark_info: &StarkInfo = self.stark_info.as_ref().unwrap();

        if stage_id == 1 {
            debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, 0);
            timer_start!(STARK_INITIALIZATION);

            //initialize the transcript if has not been initialized by another prover
            let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };
            proof_ctx.transcript.get_or_insert_with(|| {
                FFITranscript::new(
                    p_stark,
                    element_type,
                    self.merkle_tree_arity.unwrap(),
                    self.merkle_tree_custom.unwrap(),
                )
            });
            //initialize the challenges if have not been initialized by another prover
            proof_ctx.challenges.get_or_insert_with(|| {
                vec![F::zero(); stark_info.challenges_map.as_ref().unwrap().len() * Self::FIELD_EXTENSION as usize]
            });

            self.evals = vec![F::zero(); stark_info.ev_map.len() * Self::FIELD_EXTENSION as usize];

            self.subproof_values =
                vec![F::zero(); stark_info.n_subproof_values as usize * Self::FIELD_EXTENSION as usize];

            self.p_proof = Some(fri_proof_new_c(p_stark));

            self.p_params = Some(steps_params_new_c(
                p_stark,
                proof_ctx.challenges.as_ref().unwrap().as_ptr() as *mut c_void,
                self.subproof_values.as_ptr() as *mut c_void,
                self.evals.as_ptr() as *mut c_void,
                proof_ctx.public_inputs.as_ptr() as *mut c_void,
            ));

            let high_bound = *stark_info.map_sections_n.get("cm1").unwrap() as usize;
            for i in 0..high_bound {
                set_symbol_calculated_c(p_stark, OpType::Cm.as_integer(), i as u64);
            }

            for i in 0..stark_info.n_publics as usize {
                set_symbol_calculated_c(p_stark, OpType::Public.as_integer(), i as u64);
            }

            timer_stop_and_log!(STARK_INITIALIZATION);

            //--------------------------------
            // 0.- Add const root and publics to transcript
            //--------------------------------
            //timer_start!(STARK_COMMIT_STAGE_0);
            // challenged are added to transcript within the function callculate challenges,
            // stark commit stage 0 is not needed anymore
            //timer_stop_and_log!(STARK_COMMIT_STAGE_0);
        }

        debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, stage_id);

        timer_start!(STARK_COMMIT_STAGE_, stage_id);

        let p_params = self.p_params.unwrap();
        let p_proof = self.p_proof.unwrap();
        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };

        if stage_id <= proof_ctx.pilout.num_stages() {
            compute_stage_expressions_c(p_stark, element_type, stage_id as u64, p_params, p_proof, self.p_steps);
        } else {
            calculate_expression_c(p_stark, std::ptr::null_mut(), stark_info.c_exp_id, p_params, self.p_steps, true);
        }

        commit_stage_c(p_stark, element_type, stage_id as u64, p_params, p_proof);

        timer_stop_and_log!(STARK_COMMIT_STAGE_, stage_id);

        if stage_id <= self.num_stages() + 1 {
            ProverStatus::CommitStage
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<F>) -> ProverStatus {
        let last_stage_id = 2 + self.stark_info.as_ref().unwrap().stark_struct.steps.len() as u32 + 1;

        if opening_id == 1 {
            self.compute_evals(opening_id, proof_ctx);
        } else if opening_id == 2 {
            self.compute_fri_pol(opening_id, proof_ctx);
        } else if opening_id < last_stage_id {
            self.compute_fri_folding(opening_id, proof_ctx);
        } else if opening_id == last_stage_id {
            self.compute_fri_queries(opening_id, proof_ctx);
        } else {
            panic!("Opening stage not implemented");
        }

        if opening_id == last_stage_id {
            ProverStatus::StagesCompleted
        } else {
            ProverStatus::OpeningStage
        }
    }

    fn get_commit_stage_root_challenge_256(&self, stage_id: u32) -> Option<[u64; 4]> {
        let p_root_challenge = get_proof_root_c(self.p_proof.unwrap(), stage_id as u64 - 1, 0);
        let p_root_challenge = unsafe { &*(p_root_challenge as *mut u64) };

        let challenge: &[u64] = unsafe { slice::from_raw_parts(p_root_challenge, self.n_field_elements as usize) };

        let challenge: [u64; 4] = challenge.try_into().unwrap_or_else(|_| {
            panic!("Expected a slice of length 4");
        });

        Some(challenge)
    }

    fn get_opening_stage_root_challenge_256(&self, opening_id: u32) -> Option<[u64; 4]> {
        let stark_info = self.stark_info.as_ref().unwrap();
        let last_stage_id = 2 + stark_info.stark_struct.steps.len() as u32 + 1;
        let p_stark = self.p_stark.unwrap();

        let mut challenge = vec![0u64; self.n_field_elements];

        match opening_id {
            1 => {
                // Compute evals
                calculate_hash_c(
                    p_stark,
                    challenge.as_ptr() as *mut c_void,
                    self.evals.as_ptr() as *mut c_void,
                    stark_info.ev_map.len() as u64 * Self::FIELD_EXTENSION as u64,
                );
            }

            // Compute FRI polynomial => Nothing to be done
            // Note: After computing FRI polynomial add a new challenge is not needed because during the first ietration of FRI folding any challenge is needed
            2 => {
                return None;
            }

            opening_id if opening_id < last_stage_id => {
                // Compute FRI folding
                let step = opening_id - 3;
                if step < stark_info.stark_struct.steps.len() as u32 - 1 {
                    let p_root_challenge = fri_proof_get_tree_root_c(self.p_proof.unwrap(), step as u64 + 1, 0);
                    let p_root_challenge = unsafe { &*(p_root_challenge as *mut u64) };

                    unsafe {
                        std::ptr::copy_nonoverlapping(p_root_challenge, challenge.as_mut_ptr(), self.n_field_elements);
                    }
                } else {
                    let p_fri_pol = get_steps_params_field_c(self.p_params.unwrap(), "f_2ns");
                    let n_elements: usize =
                        (1 << stark_info.stark_struct.steps[step as usize].n_bits) * Self::FIELD_EXTENSION;

                    calculate_hash_c(p_stark, challenge.as_mut_ptr() as *mut c_void, p_fri_pol, n_elements as u64);
                }
            }

            // Compute FRI Queries => Last stage, nothing to be done
            opening_id if opening_id == last_stage_id => {
                return None;
            }
            _ => {
                panic!("Opening stage not implemented");
            }
        }

        let challenge: [u64; 4] = challenge.try_into().unwrap_or_else(|_| {
            panic!("Expected a slice of length 4");
        });

        Some(challenge)
    }

    fn add_root_challenge_256_to_transcript(&mut self, root_challenge: [u64; 4]) {
        //self.transcript.as_mut().unwrap().add_elements(root_challenge.as_ptr() as *mut c_void, 4);
    }

    fn get_map_offsets(&self, stage: &str, is_extended: bool) -> u64 {
        get_map_offsets_c(self.p_starkinfo, stage, is_extended)
    }

    fn add_challenges_to_transcript(&self, stage: u64, proof_ctx: &mut ProofCtx<F>) {
        let p_stark: *mut std::ffi::c_void = self.p_stark.unwrap();
        let transcript: &FFITranscript = proof_ctx.transcript.as_ref().unwrap();

        if stage <= (Self::num_stages(&self) + 1) as u64 {
            let mut tree_index = 0;
            let root = vec![F::zero(); self.n_field_elements];
            if stage == 0 {
                let stark_info: &StarkInfo = self.stark_info.as_ref().unwrap();
                tree_index = stark_info.n_stages as u64 + 1;
            } else {
                tree_index = stage - 1;
            }
            treesGL_get_root_c(p_stark, tree_index, root.as_ptr() as *mut c_void);
            transcript.add_elements(root.as_ptr() as *mut c_void, self.n_field_elements);
        } else {
            if stage == (Self::num_stages(&self) + 2) as u64 {
                //TODO: hardcoded, option no hash must be included
                let hash: Vec<F> = vec![F::zero(); self.n_field_elements];
                calculate_hash_c(
                    p_stark,
                    hash.as_ptr() as *mut c_void,
                    self.evals.as_ptr() as *mut c_void,
                    self.evals.len() as u64,
                );
                transcript.add_elements(hash.as_ptr() as *mut c_void, self.n_field_elements);
            }
        }
    }

    //TODO: This funciton could leave outside the prover trait, for now is confortable to get the hash and the configs
    fn add_publics_to_transcript(&self, proof_ctx: &mut ProofCtx<F>) {
        let p_stark: *mut std::ffi::c_void = self.p_stark.unwrap();
        let stark_info: &StarkInfo = self.stark_info.as_ref().unwrap();
        let transcript: &FFITranscript = proof_ctx.transcript.as_ref().unwrap();
        if stark_info.stark_struct.hash_commits {
            let hash: Vec<F> = vec![F::zero(); self.n_field_elements];
            calculate_hash_c(
                p_stark,
                hash.as_ptr() as *mut c_void,
                proof_ctx.public_inputs.as_ptr() as *mut c_void,
                stark_info.n_publics,
            );
            transcript.add_elements(hash.as_ptr() as *mut c_void, self.n_field_elements);
        } else {
            let mut inputs_: Vec<u64> = vec![25, 0, 2, 9]; //TODO: harcoded
            let inputs_ptr: *mut c_void = inputs_.as_mut_ptr() as *mut c_void;

            transcript.add_elements(inputs_ptr, stark_info.n_publics as usize);
        }
    }

    // fn get_subproof_values(&self) -> Vec<T> {
    //     self.subproof_values.clone()
    // }

    fn get_challenges(&self, stage_id: u32, proof_ctx: &mut ProofCtx<F>) {
        if stage_id == 1 {
            return;
        }
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_stark = self.p_stark.unwrap();
        let transcript = proof_ctx.transcript.as_ref().unwrap();

        let challenges_map = stark_info.challenges_map.as_ref().unwrap();

        let challenges = proof_ctx.challenges.as_ref().unwrap();
        for i in 0..challenges_map.len() {
            if challenges_map[i].stage == stage_id as u64 {
                transcript.get_challenge(&challenges[i * Self::FIELD_EXTENSION] as *const F as *mut c_void);
                set_symbol_calculated_c(p_stark, OpType::Challenge.as_integer(), i as u64);
            }
        }
    }
}

impl<F: AbstractField> StarkProver<F> {
    // Return the total number of elements needed to compute the STARK
    pub fn get_total_bytes(&self) -> usize {
        get_map_totaln_c(self.p_starkinfo) as usize * std::mem::size_of::<F>()
    }

    fn compute_evals(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_stark = self.p_stark.unwrap();
        let p_params = self.p_params.unwrap();
        let p_proof = self.p_proof.unwrap();

        debug!("{}: ··· Computing evaluations", Self::MY_NAME);

        //self.get_challenges(stark_info.n_stages as u32 + 2, proof_ctx);

        compute_evals_c(p_stark, p_params, p_proof);
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_stark = self.p_stark.unwrap();
        let p_params = self.p_params.unwrap();

        debug!("{}: ··· Computing FRI Polynomial", Self::MY_NAME);

        //self.get_challenges(stark_info.n_stages as u32 + 3, proof_ctx);

        self.p_fri_pol = Some(compute_fri_pol_c(p_stark, stark_info.n_stages as u64 + 2, p_params, self.p_steps));
    }

    fn compute_fri_folding(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<F>) {
        let p_stark = self.p_stark.unwrap();
        let p_proof = self.p_proof.unwrap();
        let step = opening_id - 3;

        debug!("{}: ··· Computing FRI folding", Self::MY_NAME);

        //TODO: hardcoded!!
        let mut challenge = vec![F::zero(); Self::FIELD_EXTENSION];
        challenge[0] = proof_ctx.challenges.as_ref().unwrap()[12].clone();
        challenge[1] = proof_ctx.challenges.as_ref().unwrap()[13].clone();
        challenge[2] = proof_ctx.challenges.as_ref().unwrap()[14].clone();

        compute_fri_folding_c(
            p_stark,
            p_proof,
            get_steps_params_field_c(self.p_params.unwrap(), "f_2ns"),
            step as u64,
            challenge.as_ptr() as *mut c_void,
        );
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<F>) {
        /*let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();

        debug!("{}: ··· Computing FRI queries", Self::MY_NAME);

        let mut fri_queries = vec![u64::default(); stark_info.stark_struct.n_queries as usize];
        let challenge = vec![F::zero(); Self::FIELD_EXTENSION];

        transcript.get_challenge(challenge.as_ptr() as *mut c_void);

        let element_type = if type_name::<F>() == type_name::<Goldilocks>() { 1 } else { 0 };
        let transcript_permutation = FFITranscript::new(
            p_stark,
            element_type,
            self.merkle_tree_arity.unwrap(),
            self.merkle_tree_custom.unwrap(),
        );

        transcript_permutation.add_elements(challenge.as_ptr() as *mut c_void, Self::FIELD_EXTENSION);

        transcript_permutation.get_permutations(
            fri_queries.as_mut_ptr(),
            stark_info.stark_struct.n_queries,
            stark_info.stark_struct.steps[0].n_bits,
        );

        compute_fri_queries_c(p_stark, p_proof, fri_queries.as_mut_ptr());
        */
        // NOTE: Check!
        // proof_ctx.proof = p_proof;
    }
}
