use goldilocks::{AbstractField, Goldilocks};
use proofman::proof_manager::ProverStatus;
use transcript::FFITranscript;
use std::any::type_name;

use proofman::provers_manager::Prover;
use log::debug;
use util::{timer_start, timer_stop_and_log};
use zkevm_lib_c::ffi::*;
use proofman::ProofCtx;
use crate::stark_info::{OpType, StarkInfo};
use crate::stark_prover_settings::StarkProverSettings;

use std::os::raw::c_void;

pub struct StarkProver<T: AbstractField> {
    initialized: bool,
    config: StarkProverSettings,
    p_chelpers: *mut c_void,
    p_steps: *mut c_void,
    ptr: *mut c_void,
    pub p_stark: Option<*mut c_void>,
    p_params: Option<*mut c_void>,
    p_proof: Option<*mut c_void>,
    transcript: Option<FFITranscript>,
    p_fri_pol: Option<*mut c_void>,
    stark_info: Option<StarkInfo>,
    p_starkinfo: *mut c_void,

    evals: Vec<T>,
    challenges: Vec<T>,
    subproof_values: Vec<T>,
    x_div_x_sub_xi: Vec<T>,

    hash_size: usize,
    merkle_tree_arity: Option<u64>,
    merkle_tree_custom: Option<bool>,

    // Pointers to the bool vector values inside the C++ code
    p_publics_calculated: *mut c_void,
    p_const_calculated: *mut c_void,
    p_subproof_values_calculated: *mut c_void,
    p_challenges_calculated: *mut c_void,
    p_witnesses_calculated: *mut c_void,

    phantom: std::marker::PhantomData<T>,
}

impl<T: AbstractField> StarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    const HASH_SIZE: usize = 4;
    const FIELD_EXTENSION: usize = 3;

    pub fn new(
        config: StarkProverSettings,
        p_starkinfo: *mut c_void,
        p_chelpers: *mut c_void,
        p_steps: *mut c_void,
        ptr: *mut c_void,
    ) -> Self {
        Self {
            initialized: false,
            config,
            p_chelpers,
            p_steps,
            ptr,
            p_stark: None,
            p_params: None,
            p_proof: None,
            transcript: None,
            p_fri_pol: None,
            stark_info: None,
            p_starkinfo,
            evals: Vec::new(),
            challenges: Vec::new(),
            subproof_values: Vec::new(),
            x_div_x_sub_xi: Vec::new(),
            hash_size: 0,
            merkle_tree_arity: None,
            merkle_tree_custom: None,
            p_publics_calculated: std::ptr::null_mut(),
            p_const_calculated: std::ptr::null_mut(),
            p_subproof_values_calculated: std::ptr::null_mut(),
            p_challenges_calculated: std::ptr::null_mut(),
            p_witnesses_calculated: std::ptr::null_mut(),
            phantom: std::marker::PhantomData,
        }
    }

    pub fn get_stark_info(&self) -> *mut c_void {
        get_stark_info_c(self.p_stark.unwrap())
    }
}

impl<T: AbstractField> Prover<T> for StarkProver<T> {
    fn build(&mut self) {
        timer_start!(ESTARK_PROVER_NEW);

        let p_config = config_new_c(&self.config.current_path);
        let stark_info_json = std::fs::read_to_string(&self.config.stark_info_filename)
            .expect(format!("Failed to read file {}", &self.config.stark_info_filename).as_str());

        self.stark_info = Some(StarkInfo::from_json(&stark_info_json));

        let p_stark = starks_new_c(
            p_config,
            self.config.const_pols_filename.as_str(),
            self.config.map_const_pols_file,
            self.config.const_tree_filename.as_str(),
            self.p_starkinfo,
            self.p_chelpers,
            self.ptr,
        );

        self.p_stark = Some(p_stark);

        let stark_info = self.stark_info.as_ref().unwrap();

        self.p_publics_calculated = get_vector_pointer_c(p_stark, "publicsCalculated");
        self.p_const_calculated = get_vector_pointer_c(p_stark, "constsCalculated");
        self.p_subproof_values_calculated = get_vector_pointer_c(p_stark, "subProofValuesCalculated");
        self.p_challenges_calculated = get_vector_pointer_c(p_stark, "challengesCalculated");
        self.p_witnesses_calculated = get_vector_pointer_c(p_stark, "witnessCalculated");

        let element_type = if type_name::<T>() == type_name::<Goldilocks>() { 1 } else { 0 };

        if stark_info.stark_struct.verification_hash_type == "BN128" {
            self.hash_size = 1;
            self.merkle_tree_arity = Some(stark_info.stark_struct.merkle_tree_arity);
            self.merkle_tree_custom = Some(stark_info.stark_struct.merkle_tree_custom);
        } else {
            self.hash_size = Self::HASH_SIZE;
            self.merkle_tree_arity = Some(2);
            self.merkle_tree_custom = Some(true);
        }

        self.transcript = Some(FFITranscript::new(
            p_stark,
            element_type,
            self.merkle_tree_arity.unwrap(),
            self.merkle_tree_custom.unwrap(),
        ));

        clean_symbols_calculated_c(p_stark);

        self.initialized = true;

        timer_stop_and_log!(ESTARK_PROVER_NEW);
    }

    fn commit_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
        debug!("{}: ··· Computing commit stage {}", Self::MY_NAME, stage_id);

        if !self.initialized {
            self.build();
        }

        let transcript = self.transcript.as_ref().unwrap();
        let p_stark = self.p_stark.unwrap();

        if stage_id == 1 {
            timer_start!(STARK_INITIALIZATION);

            let stark_info = self.stark_info.as_ref().unwrap();

            let n_extended = 1 << stark_info.stark_struct.n_bits_ext;

            self.evals = vec![T::zero(); stark_info.ev_map.len() * Self::FIELD_EXTENSION as usize];
            self.challenges =
                vec![T::zero(); stark_info.challenges_map.as_ref().unwrap().len() * Self::FIELD_EXTENSION as usize];
            self.subproof_values =
                vec![T::zero(); stark_info.n_subproof_values as usize * Self::FIELD_EXTENSION as usize];
            self.x_div_x_sub_xi =
                vec![T::zero(); stark_info.opening_points.len() * n_extended * Self::FIELD_EXTENSION as usize];

            self.p_proof = Some(fri_proof_new_c(p_stark));

            self.p_params = Some(steps_params_new_c(
                p_stark,
                self.challenges.as_ptr() as *mut c_void,
                self.subproof_values.as_ptr() as *mut c_void,
                self.evals.as_ptr() as *mut c_void,
                self.x_div_x_sub_xi.as_ptr() as *mut c_void,
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
            timer_start!(STARK_COMMIT_STAGE_0);

            let verkey = vec![T::zero(); self.hash_size];
            treesGL_get_root_c(p_stark, stark_info.n_stages + 1, verkey.as_ptr() as *mut c_void);

            transcript.add_elements(verkey.as_ptr() as *mut c_void, self.hash_size);

            if stark_info.stark_struct.hash_commits {
                let hash = vec![T::zero(); self.hash_size];
                calculate_hash_c(
                    p_stark,
                    hash.as_ptr() as *mut c_void,
                    proof_ctx.public_inputs.as_ptr() as *mut c_void,
                    stark_info.n_publics,
                );
                transcript.add_elements(hash.as_ptr() as *mut c_void, self.hash_size);
            } else {
                transcript.add_elements(proof_ctx.public_inputs.as_ptr() as *mut c_void, stark_info.n_publics as usize)
            }

            timer_stop_and_log!(STARK_COMMIT_STAGE_0);
        }

        let p_params = self.p_params.unwrap();
        let p_proof = self.p_proof.unwrap();

        timer_start!(STARK_COMMIT_STAGE_, stage_id);
        let element_type = if type_name::<T>() == type_name::<Goldilocks>() { 1 } else { 0 };
        compute_stage_c(
            p_stark,
            element_type,
            stage_id as u64,
            p_params,
            p_proof,
            transcript.p_transcript,
            self.p_steps,
        );
        timer_stop_and_log!(STARK_COMMIT_STAGE_, stage_id);

        ProverStatus::StagesPending
    }

    fn opening_stage(&mut self, opening_id: u32, proof_ctx: &mut ProofCtx<T>) -> ProverStatus {
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
            ProverStatus::StagesPending
        }
    }
}

impl<T: AbstractField> StarkProver<T> {
    fn compute_evals(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_params = self.p_params.unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();

        debug!("{}: ··· Computing evaluations", Self::MY_NAME);

        let challenges_map = stark_info.challenges_map.as_ref().unwrap();

        for i in 0..challenges_map.len() {
            if challenges_map[i].stage_num == stark_info.n_stages + 2 {
                transcript.get_challenge(&self.challenges[i * Self::FIELD_EXTENSION] as *const T as *mut c_void);
                set_symbol_calculated_c(p_stark, OpType::Challenge.as_integer(), i as u64);
            }
        }

        compute_evals_c(p_stark, p_params, p_proof);

        if stark_info.stark_struct.hash_commits {
            let hash = vec![T::zero(); self.hash_size];
            calculate_hash_c(
                p_stark,
                hash.as_ptr() as *mut c_void,
                self.evals.as_ptr() as *mut c_void,
                stark_info.ev_map.len() as u64 * Self::FIELD_EXTENSION as u64,
            );
            transcript.add_elements(hash.as_ptr() as *mut c_void, self.hash_size);
        } else {
            transcript.add_elements(
                self.evals.as_ptr() as *mut c_void,
                stark_info.ev_map.len() as usize * Self::FIELD_EXTENSION,
            );
        }
    }

    fn compute_fri_pol(&mut self, _opening_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let p_params = self.p_params.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let n_stages = stark_info.n_stages;
        let transcript = self.transcript.as_ref().unwrap();

        debug!("{}: ··· Computing FRI Polynomial", Self::MY_NAME);

        let challenges_map = stark_info.challenges_map.as_ref().unwrap();

        for i in 0..challenges_map.len() {
            if challenges_map[i].stage_num == stark_info.n_stages + 3 {
                transcript.get_challenge(&self.challenges[i * Self::FIELD_EXTENSION] as *const T as *mut c_void);
                set_symbol_calculated_c(p_stark, OpType::Challenge.as_integer(), i as u64);
            }
        }

        self.p_fri_pol = Some(compute_fri_pol_c(p_stark, n_stages + 2, p_params, self.p_steps));
    }

    fn compute_fri_folding(&mut self, opening_id: u32, _proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();
        let step = opening_id - 3;

        debug!("{}: ··· Computing FRI folding", Self::MY_NAME);

        let challenge = vec![T::zero(); Self::FIELD_EXTENSION];
        transcript.get_challenge(challenge.as_ptr() as *mut c_void);

        compute_fri_folding_c(
            p_stark,
            p_proof,
            get_steps_params_field_c(self.p_params.unwrap(), "f_2ns"),
            step as u64,
            challenge.as_ptr() as *mut c_void,
        );

        if step < stark_info.stark_struct.steps.len() as u32 - 1 {
            let root = fri_proof_get_tree_root_c(p_proof, step as u64 + 1, 0);
            transcript.add_elements(root, self.hash_size);
        } else {
            let p_fri_pol = get_steps_params_field_c(self.p_params.unwrap(), "f_2ns");
            let n_elements: usize = (1 << stark_info.stark_struct.steps[step as usize].n_bits) * Self::FIELD_EXTENSION;
            if stark_info.stark_struct.hash_commits {
                let hash = vec![T::zero(); self.hash_size];
                // TODO! get params.f_2ns
                calculate_hash_c(p_stark, hash.as_ptr() as *mut c_void, p_fri_pol, n_elements as u64);
                transcript.add_elements(hash.as_ptr() as *mut c_void, self.hash_size);
            } else {
                transcript.add_elements(p_fri_pol, n_elements as usize);
            }
        }
    }

    fn compute_fri_queries(&mut self, _opening_id: u32, proof_ctx: &mut ProofCtx<T>) {
        let p_stark = self.p_stark.unwrap();
        let stark_info = self.stark_info.as_ref().unwrap();
        let p_proof = self.p_proof.unwrap();
        let transcript = self.transcript.as_ref().unwrap();

        debug!("{}: ··· Computing FRI queries", Self::MY_NAME);

        let mut fri_queries = vec![u64::default(); stark_info.stark_struct.n_queries as usize];
        let challenge = vec![T::zero(); Self::FIELD_EXTENSION];

        transcript.get_challenge(challenge.as_ptr() as *mut c_void);

        let element_type = if type_name::<T>() == type_name::<Goldilocks>() { 1 } else { 0 };
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

        proof_ctx.proof = p_proof;
    }
}
