use goldilocks::{AbstractField, Goldilocks};
use std::any::type_name;

use proofman::provers_manager::Prover;
use log::{info, debug};
use util::{timer_start, timer_stop_and_log};
use zkevm_lib_c::ffi::*;
use proofman::proof_ctx::ProofCtx;
use crate::stark_info::StarkInfo;
use crate::estark_prover_settings::EStarkProverSettings;

pub struct EStarkProver<T: AbstractField> {
    initialized: bool,
    config: EStarkProverSettings,
    p_steps: *mut ::std::os::raw::c_void,
    ptr: *mut std::os::raw::c_void,
    pub p_stark: Option<*mut ::std::os::raw::c_void>,
    stark_info: Option<StarkInfo>,
    phantom: std::marker::PhantomData<T>,
}

impl<T: AbstractField> EStarkProver<T> {
    const MY_NAME: &'static str = "estrkPrv";

    pub fn new(
        config: EStarkProverSettings,
        p_steps: *mut std::os::raw::c_void,
        ptr: *mut std::os::raw::c_void,
    ) -> Self {
        Self {
            initialized: false,
            config,
            p_steps,
            ptr,
            p_stark: None,
            stark_info: None,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn get_stark_info(&self) -> *mut ::std::os::raw::c_void {
        get_stark_info_c(self.p_stark.unwrap())
    }
}

impl<T: AbstractField> Prover<T> for EStarkProver<T> {
    fn build(&mut self) {
        timer_start!(ESTARK_PROVER_NEW);

        let p_config = config_new_c(&self.config.current_path);
        let stark_info_json = std::fs::read_to_string(&self.config.stark_info_filename)
            .expect(format!("Failed to read file {}", &self.config.stark_info_filename).as_str());
        self.stark_info = Some(StarkInfo::from_json(&stark_info_json));

        self.p_stark = Some(starks_new_c(
            p_config,
            self.config.const_pols_filename.as_str(),
            self.config.map_const_pols_file,
            self.config.const_tree_filename.as_str(),
            self.config.stark_info_filename.as_str(),
            self.config.chelpers_filename.as_str(),
            self.ptr,
        ));

        self.initialized = true;

        timer_stop_and_log!(ESTARK_PROVER_NEW);
    }

    fn compute_stage(&mut self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        if !self.initialized {
            self.build();
        }

        info!("{}: --> eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        const HASH_SIZE: u64 = 4;
        const FIELD_EXTENSION: u64 = 3;

        timer_start!(STARK_COMPUTE_STAGE);

        timer_start!(STARK_INITIALIZATION);

        let element_type = if type_name::<T>() == type_name::<Goldilocks>() { 1 } else { 0 };
        let p_transcript = transcript_new_c(element_type);

        let stark_info = self.stark_info.as_ref().unwrap();

        let p_evals = polinomial_new_c(stark_info.ev_map.len() as u64, FIELD_EXTENSION, "");
        let p_challenges = polinomial_new_c(stark_info.n_challenges.unwrap(), FIELD_EXTENSION, "");
        let p_subproof_values = polinomial_new_c(stark_info.n_subair_values.unwrap(), FIELD_EXTENSION, "");

        let n_extended = 1 << stark_info.stark_struct.n_bits_ext;
        let p_x_div_x_sub_xi = polinomial_new_c(
            stark_info.opening_points.as_ref().unwrap().len() as u64 * n_extended,
            FIELD_EXTENSION,
            "",
        );

        let hash_size = if stark_info.stark_struct.verification_hash_type == "BN128" { 1 } else { HASH_SIZE };
        let verkey = vec![T::zero(); hash_size as usize];

        let p_stark = self.p_stark.unwrap();
        treesGL_get_root_c(p_stark, stark_info.n_stages.unwrap() + 1, verkey.as_ptr() as *mut std::os::raw::c_void);

        let p_proof = fri_proof_new_c(p_stark);

        let p_params = steps_params_new_c(
            p_stark,
            p_challenges,
            p_subproof_values,
            p_evals,
            p_x_div_x_sub_xi,
            proof_ctx.public_inputs.as_ptr() as *mut std::os::raw::c_void,
        );

        timer_stop_and_log!(STARK_INITIALIZATION);

        //--------------------------------
        // 0.- Add const root and publics to transcript
        //--------------------------------
        timer_start!(STARK_STEP_0);

        transcript_add_c(p_transcript, verkey.as_ptr() as *mut std::os::raw::c_void, HASH_SIZE);

        transcript_add_c(
            p_transcript,
            proof_ctx.public_inputs.as_ptr() as *mut std::os::raw::c_void,
            stark_info.n_publics,
        );

        timer_stop_and_log!(STARK_STEP_0);

        //--------------------------------
        // 1.- Compute stages
        //--------------------------------

        for step in 1..=stark_info.n_stages.unwrap() + 1 {
            timer_start!(STARK_COMMIT_STAGE_XX);
            compute_stage_c(p_stark, element_type, step, p_params, p_proof, p_transcript, self.p_steps);
            timer_stop_and_log!(STARK_COMMIT_STAGE_XX);
        }

        //--------------------------------
        // 2. Compute Evals
        //--------------------------------
        timer_start!(STARK_COMMIT_STAGE_YY);

        get_challenge_c(p_stark, p_transcript, polinomial_get_p_element_c(p_challenges, 7));

        compute_evals_c(p_stark, p_params, p_proof);

        transcript_add_polinomial_c(p_transcript, p_evals);

        get_challenge_c(p_stark, p_transcript, polinomial_get_p_element_c(p_challenges, 5));
        get_challenge_c(p_stark, p_transcript, polinomial_get_p_element_c(p_challenges, 6));

        // Polinomial* friPol = computeFRIPol(starkInfo.nStages + 2, params, chelpersSteps);

        timer_stop_and_log!(STARK_COMMIT_STAGE_YY);

        //--------------------------------
        // 3. Compute FRI
        //--------------------------------
        timer_start!(STARK_STEP_FRI);

        let p_fri_pol = compute_fri_pol_c(p_stark, stark_info.n_stages.unwrap() + 2, p_params, self.p_steps);

        for step in 0..stark_info.stark_struct.steps.len() {
            let challenge = polinomial_new_c(1, FIELD_EXTENSION, "");
            get_challenge_c(p_stark, p_transcript, polinomial_get_p_element_c(challenge, 0));

            compute_fri_folding_c(p_stark, p_proof, p_fri_pol, step as u64, challenge);
            if step < stark_info.stark_struct.steps.len() - 1 {
                let root = fri_proof_get_tree_root_c(p_proof, step as u64 + 1, 0);
                transcript_add_c(p_transcript, root, HASH_SIZE);
            } else {
                transcript_add_polinomial_c(p_transcript, p_fri_pol);
            }
        }

        let mut fri_queries = vec![0u64; stark_info.stark_struct.n_queries as usize];

        get_permutations_c(
            p_transcript,
            fri_queries.as_mut_ptr(),
            stark_info.stark_struct.n_queries,
            stark_info.stark_struct.steps[0].n_bits,
        );

        compute_fri_queries_c(p_stark, p_proof, p_fri_pol, fri_queries.as_mut_ptr());

        polinomial_free_c(p_fri_pol);
        polinomial_free_c(p_subproof_values);
        polinomial_free_c(p_challenges);

        polinomial_free_c(p_evals);

        timer_stop_and_log!(STARK_STEP_FRI);

        timer_stop_and_log!(STARK_COMPUTE_STAGE);

        proof_ctx.proof = Some(p_proof);

        steps_params_free_c(p_params);
        transcript_free_c(p_transcript, element_type);

        info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);
    }
}
