#![allow(non_snake_case)]

use goldilocks::Goldilocks;
use proofman::provers_manager::Prover;
use log::{info, debug};
use util::{timer_start, timer_stop_and_log};
use crate::ffi::*;
use proofman::proof_ctx::ProofCtx;
use crate::verification_key::VerificationKey;
use std::time::Instant;
use crate::stark_info::StarkInfo;
use crate::estark_prover_settings::EStarkProverSettings;

pub struct EStarkProver<T> {
    pub p_starks: *mut ::std::os::raw::c_void,
    p_steps: *mut ::std::os::raw::c_void,
    stark_info: StarkInfo,
    verkey: VerificationKey<Goldilocks>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> EStarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    pub fn new(
        config: &EStarkProverSettings,
        p_steps: *mut std::os::raw::c_void,
        ptr: *mut std::os::raw::c_void,
    ) -> Self {
        timer_start!(ESTARK_PROVER_NEW);

        let p_config = config_new_c(&config.current_path);
        let stark_info_json = std::fs::read_to_string(&config.stark_info_filename)
            .expect(format!("Failed to read file {}", &config.stark_info_filename).as_str());
        let stark_info = StarkInfo::from_json(&stark_info_json);

        let verkey_json = std::fs::read_to_string(&config.verkey_filename)
            .expect(format!("Failed to read file {}", &config.verkey_filename).as_str());
        let verkey = VerificationKey::<Goldilocks>::from_json(&verkey_json);

        let p_starks = starks_new_c(
            p_config,
            config.const_pols_filename.as_str(),
            config.map_const_pols_file,
            config.const_tree_filename.as_str(),
            config.stark_info_filename.as_str(),
            ptr,
        );

        timer_stop_and_log!(ESTARK_PROVER_NEW);

        Self { p_starks, p_steps, stark_info, verkey, phantom: std::marker::PhantomData }
    }

    pub fn get_stark_info(&self) -> *mut ::std::os::raw::c_void {
        get_stark_info_c(self.p_starks)
    }
}

impl<T> Prover<T> for EStarkProver<T> {
    fn compute_stage(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        self.compute_stage_new(stage_id, proof_ctx)
        // timer_start!(STARK_GENPROOF);

        // let n_bits = self.stark_info.stark_struct.steps[self.stark_info.stark_struct.steps.len() - 1].n_bits;
        // let n_trees = self.stark_info.stark_struct.steps.len() as u64;
        // let n_publics = self.stark_info.n_publics;
        // let eval_size = self.stark_info.ev_map.len() as u64;
        // const FIELD_EXTENSION: u64 = 3;

        // let p_fri_proof = fri_proof_new_c(1 << n_bits, FIELD_EXTENSION, n_trees, eval_size, n_publics);

        // starks_genproof_c::<T>(self.p_starks, p_fri_proof, &proof_ctx.public_inputs, &self.verkey, self.p_steps);

        // timer_stop_and_log!(STARK_GENPROOF);

        // proof_ctx.proof = Some(p_fri_proof);
        // info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        // return;
    }
}

impl<T> EStarkProver<T> {
    fn compute_stage_new(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        const HASH_SIZE: u64 = 4;
        const FIELD_EXTENSION: u64 = 3;

        timer_start!(STARK_COMPUTE_STAGE);

        timer_start!(STARK_GENPROOF_INITIALIZATION);

        let n = 1 << self.stark_info.stark_struct.n_bits;
        let n_extended = 1 << self.stark_info.stark_struct.n_bits_ext;
        let n_rows_step_batch = get_num_rows_step_batch_c(self.p_starks);

        let p_transcript = transcript_new_c();

        let p_evals = polinomial_new_c(self.stark_info.ev_map.len() as u64, FIELD_EXTENSION, "");
        let p_x_div_x_sub_xi = vec![polinomial_new_void_c(); self.stark_info.opening_points.len()];
        let p_challenges = polinomial_new_c(self.stark_info.n_challenges, FIELD_EXTENSION, "");

        let p_fri_proof = fri_proof_new_c(self.p_starks);

        let p_params = steps_params_new_c(
            self.p_starks,
            p_challenges,
            p_evals,
            p_x_div_x_sub_xi[0],
            p_x_div_x_sub_xi[1],
            proof_ctx.public_inputs.as_ptr() as *mut std::os::raw::c_void,
        );

        timer_stop_and_log!(STARK_GENPROOF_INITIALIZATION);

        //--------------------------------
        // 0.- Add const root and publics to transcript
        //--------------------------------

        transcript_add_c(p_transcript, self.verkey.const_root.as_ptr() as *mut std::os::raw::c_void, HASH_SIZE);
        transcript_add_c(
            p_transcript,
            proof_ctx.public_inputs.as_ptr() as *mut std::os::raw::c_void,
            self.stark_info.n_publics,
        );

        //--------------------------------
        // 1.- Calculate Stage 1
        //--------------------------------
        timer_start!(STARK_STEP_1);
        let mut step = 1;

        extend_and_merkelize_c(self.p_starks, step, p_params, p_fri_proof);

        let root = fri_proof_get_root_c(p_fri_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_1);

        //--------------------------------
        // 2.- Calculate plookups h1 and h2
        //--------------------------------
        timer_start!(STARK_STEP_2);
        step = 2;

        get_challenges_c(p_transcript, p_challenges, self.stark_info.num_challenges[step as usize - 1], 0);

        calculate_expressions_c(self.p_starks, "step2prev", n_rows_step_batch, self.p_steps, p_params, n);

        calculate_h1_h2_c(self.p_starks, p_params);

        extend_and_merkelize_c(self.p_starks, step, p_params, p_fri_proof);

        let root = fri_proof_get_root_c(p_fri_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_2);

        //--------------------------------
        // 3.- Compute Z polynomials
        //--------------------------------
        timer_start!(STARK_STEP_3);
        step = 3;

        get_challenges_c(p_transcript, p_challenges, self.stark_info.num_challenges[step as usize - 1], 2);

        calculate_expressions_c(self.p_starks, "step3prev", n_rows_step_batch, self.p_steps, p_params, n);

        calculate_z_c(self.p_starks, p_params);

        calculate_expressions_c(self.p_starks, "step3", n_rows_step_batch, self.p_steps, p_params, n);

        extend_and_merkelize_c(self.p_starks, step, p_params, p_fri_proof);

        let root = fri_proof_get_root_c(p_fri_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_3);

        //--------------------------------
        // 4. Compute C Polynomial
        //--------------------------------
        timer_start!(STARK_STEP_4);
        step = 4;

        get_challenges_c(p_transcript, p_challenges, 1, 4);

        calculate_expressions_c(self.p_starks, "step42ns", n_rows_step_batch, self.p_steps, p_params, n_extended);

        compute_q_c(self.p_starks, p_params, p_fri_proof);

        let root = fri_proof_get_root_c(p_fri_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_4);

        //--------------------------------
        // 5. Compute Evals
        //--------------------------------
        timer_start!(STARK_STEP_5);

        get_challenges_c(p_transcript, p_challenges, 1, 7);

        compute_evals_c(self.p_starks, p_params, p_fri_proof);

        transcript_add_polinomial_c(p_transcript, p_evals);

        get_challenges_c(p_transcript, p_challenges, 2, 5);

        timer_stop_and_log!(STARK_STEP_5);

        //--------------------------------
        // 6. Compute FRI
        //--------------------------------
        timer_start!(STARK_STEP_FRI);

        let p_fri_pol = compute_fri_pol_c(self.p_starks, p_params, self.p_steps, n_rows_step_batch);

        for step in 0..self.stark_info.stark_struct.steps.len() {
            let challenge = polinomial_new_c(1, FIELD_EXTENSION, "");
            get_challenges_c(p_transcript, challenge, 1, 0);
            compute_fri_folding_c(self.p_starks, p_fri_proof, p_fri_pol, step as u64, challenge);
            if step < self.stark_info.stark_struct.steps.len() - 1 {
                let root = fri_proof_get_tree_root_c(p_fri_proof, step as u64 + 1, 0);
                transcript_add_c(p_transcript, root, HASH_SIZE);
            } else {
                transcript_add_polinomial_c(p_transcript, p_fri_pol);
            }
        }

        let mut fri_queries = vec![0u64; self.stark_info.stark_struct.n_queries as usize];

        get_permutations_c(
            p_transcript,
            fri_queries.as_mut_ptr(),
            self.stark_info.stark_struct.n_queries,
            self.stark_info.stark_struct.steps[0].n_bits,
        );

        compute_fri_queries_c(self.p_starks, p_fri_proof, p_fri_pol, fri_queries.as_mut_ptr());

        polinomial_free_c(p_fri_pol);

        timer_stop_and_log!(STARK_STEP_FRI);

        timer_stop_and_log!(STARK_COMPUTE_STAGE);

        proof_ctx.proof = Some(p_fri_proof);
        info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        return;
    }
}
