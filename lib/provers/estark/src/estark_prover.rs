use goldilocks::AbstractField;

use proofman::provers_manager::Prover;
use log::{info, debug};
use util::{timer_start, timer_stop_and_log};
use zkevm_lib_c::ffi::*;
use proofman::proof_ctx::ProofCtx;
use std::time::Instant;
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

        let stark_info = self.stark_info.as_ref().unwrap();
        let p_stark = self.p_stark.unwrap();

        let n_extended = 1 << stark_info.stark_struct.n_bits_ext;

        let p_transcript = transcript_new_c();

        let p_evals = polinomial_new_c(stark_info.ev_map.len() as u64, FIELD_EXTENSION, "");
        let p_challenges = polinomial_new_c(stark_info.n_challenges, FIELD_EXTENSION, "");

        let p_div_x_sub = polinomial_new_c(stark_info.opening_points.len() as u64 * n_extended, FIELD_EXTENSION, "");

        let p_x_div_x_sub_xi = polinomial_new_with_address_c(
            polinomial_get_p_element_c(p_div_x_sub, 0),
            n_extended,
            FIELD_EXTENSION,
            FIELD_EXTENSION,
            "",
        );

        let p_x_div_x_sub_w_xi = polinomial_new_with_address_c(
            polinomial_get_p_element_c(p_div_x_sub, n_extended),
            n_extended,
            FIELD_EXTENSION,
            FIELD_EXTENSION,
            "",
        );

        let hash_size = if stark_info.stark_struct.verification_hash_type == "BN128" { 1 } else { HASH_SIZE };

        let verkey = vec![T::zero(); hash_size as usize];
        treesGL_get_root_c(p_stark, stark_info.n_stages + 1, verkey.as_ptr() as *mut std::os::raw::c_void);

        let p_proof = fri_proof_new_c(p_stark);

        let p_params = steps_params_new_c(
            p_stark,
            p_challenges,
            p_evals,
            p_x_div_x_sub_xi,
            p_x_div_x_sub_w_xi,
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
        // 1.- Calculate Stage 1
        //--------------------------------
        timer_start!(STARK_STEP_1);
        let mut step = 1;

        extend_and_merkelize_c(p_stark, step, p_params, p_proof);

        let root = fri_proof_get_root_c(p_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_1);

        //--------------------------------
        // 2.- Calculate plookups h1 and h2
        //--------------------------------
        timer_start!(STARK_STEP_2);
        step = 2;

        get_challenges_c(
            p_stark,
            p_transcript,
            polinomial_get_p_element_c(p_challenges, 0),
            stark_info.num_challenges[step as usize - 1],
        );

        calculate_expressions_c(p_stark, "step2", p_params, self.p_steps);

        calculate_h1_h2_c(p_stark, p_params);

        extend_and_merkelize_c(p_stark, step, p_params, p_proof);

        let root = fri_proof_get_root_c(p_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_2);

        //--------------------------------
        // 3.- Compute Z polynomials
        //--------------------------------
        timer_start!(STARK_STEP_3);
        step = 3;

        get_challenges_c(
            p_stark,
            p_transcript,
            polinomial_get_p_element_c(p_challenges, 2),
            stark_info.num_challenges[step as usize - 1],
        );

        calculate_expressions_c(p_stark, "step3", p_params, self.p_steps);

        calculate_z_c(p_stark, p_params);

        calculate_expressions_c(p_stark, "step3_after", p_params, self.p_steps);

        extend_and_merkelize_c(p_stark, step, p_params, p_proof);

        let root = fri_proof_get_root_c(p_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_3);

        //--------------------------------
        // 4. Compute C Polynomial
        //--------------------------------
        timer_start!(STARK_STEP_4);
        step = 4;

        get_challenges_c(p_stark, p_transcript, polinomial_get_p_element_c(p_challenges, 4), 1);

        calculate_expressions_c(p_stark, "step4", p_params, self.p_steps);

        compute_q_c(p_stark, p_params, p_proof);

        let root = fri_proof_get_root_c(p_proof, step - 1, 0);
        transcript_add_c(p_transcript, root, HASH_SIZE);

        timer_stop_and_log!(STARK_STEP_4);

        //--------------------------------
        // 5. Compute Evals
        //--------------------------------
        timer_start!(STARK_STEP_5);

        get_challenges_c(p_stark, p_transcript, polinomial_get_p_element_c(p_challenges, 7), 1);

        compute_evals_c(p_stark, p_params, p_proof);

        transcript_add_polinomial_c(p_transcript, p_evals);

        get_challenges_c(p_stark, p_transcript, polinomial_get_p_element_c(p_challenges, 5), 2);

        timer_stop_and_log!(STARK_STEP_5);

        //--------------------------------
        // 6. Compute FRI
        //--------------------------------
        timer_start!(STARK_STEP_FRI);

        let p_fri_pol = compute_fri_pol_c(p_stark, p_params, self.p_steps);

        for step in 0..stark_info.stark_struct.steps.len() {
            let challenge = polinomial_new_c(1, FIELD_EXTENSION, "");
            get_challenges_c(p_stark, p_transcript, polinomial_get_p_element_c(challenge, 0), 1);

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

        timer_stop_and_log!(STARK_STEP_FRI);

        timer_stop_and_log!(STARK_COMPUTE_STAGE);

        proof_ctx.proof = Some(p_proof);

        steps_params_free_c(p_params);
        transcript_free_c(p_transcript);

        info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);
    }
}
