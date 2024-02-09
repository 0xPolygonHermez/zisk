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

pub struct EStarkProverConfig {
    pub current_path: String,
    pub const_pols_filename: String,
    pub map_const_pols_file: bool,
    pub const_tree_filename: String,
    pub stark_info_filename: String,
    pub verkey_filename: String,
}

pub struct EStarkProver<T> {
    pub p_starks: *mut ::std::os::raw::c_void,
    p_steps: *mut ::std::os::raw::c_void,
    stark_info: StarkInfo,
    verkey: VerificationKey<Goldilocks>,
    ptr: *mut ::std::os::raw::c_void,
    phantom: std::marker::PhantomData<T>,
}

impl<T> EStarkProver<T> {
    const MY_NAME: &'static str = "estark  ";

    pub fn new(
        config: &EStarkProverConfig,
        p_steps: *mut std::os::raw::c_void,
        ptr: *mut std::os::raw::c_void,
    ) -> Self {
        timer_start!(ESTARK_PROVER_NEW);

        let p_config = config_new_c(&config.current_path);
        let stark_info = StarkInfo::from_json(&config.stark_info_filename);

        let verkey = VerificationKey::<Goldilocks>::from_json(&config.verkey_filename);

        let p_starks = starks_new_c(
            p_config,
            config.const_pols_filename.as_str(),
            config.map_const_pols_file,
            config.const_tree_filename.as_str(),
            config.stark_info_filename.as_str(),
            ptr,
        );

        timer_stop_and_log!(ESTARK_PROVER_NEW);

        Self { p_starks, p_steps, stark_info, verkey, ptr: ptr, phantom: std::marker::PhantomData }
    }
}

impl<T> Prover<T> for EStarkProver<T> {
    fn compute_stage(&self, stage_id: u32, proof_ctx: &mut ProofCtx<T>) {
        info!("{}: --> eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        self.compute_stage_new(stage_id, proof_ctx)
        // timer_start!(STARKS_GENPROOF);

        // let n_bits = self.stark_info.stark_struct.steps[self.stark_info.stark_struct.steps.len() - 1].n_bits;
        // let n_trees = self.stark_info.stark_struct.steps.len() as u64;
        // let n_publics = self.stark_info.n_publics;
        // let eval_size = self.stark_info.ev_map.len() as u64;
        // const FIELD_EXTENSION: u64 = 3;

        // let p_fri_proof = fri_proof_new_c(1 << n_bits, FIELD_EXTENSION, n_trees, eval_size, n_publics);

        // starks_genproof_c::<T>(self.p_starks, p_fri_proof, &proof_ctx.public_inputs, &self.verkey, self.p_steps);

        // timer_stop_and_log!(STARKS_GENPROOF);

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
        const NUM_CHALLENGES: u64 = 8;

        timer_start!(STARKS_COMPUTE_STAGE);

        // Initialize vars
        timer_start!(STARK_INITIALIZATION);

        let n_bits = self.stark_info.stark_struct.n_bits;
        let n_bits_ext = self.stark_info.stark_struct.n_bits_ext;
        let n = 1 << n_bits;
        let n_extended = 1 << n_bits_ext;
        let n_rows_step_batch = get_num_rows_step_batch_c(self.p_starks);
        let p_buffer = get_pbuffer_c(self.p_starks);

        let num_commited = self.stark_info.n_cm1;
        let p_transcript = transcript_new_c();

        let p_evals = polinomial_new_c(n, FIELD_EXTENSION, "");
        let p_x_div_x_sub_xi = polinomial_new_c(n_extended, FIELD_EXTENSION, "");
        let p_x_div_x_sub_wxi = polinomial_new_c(n_extended, FIELD_EXTENSION, "");
        let p_challenges = polinomial_new_c(NUM_CHALLENGES, FIELD_EXTENSION, "");

        // Unused variable
        // let cm_pols = commit_pols_new_c(self.ptr, self.stark_info.map_deg.section[ESection::Cm1_N as usize]);

        let p_root0 = polinomial_new_c(HASH_SIZE, 1, "");
        let p_root1 = polinomial_new_c(HASH_SIZE, 1, "");
        let p_root2 = polinomial_new_c(HASH_SIZE, 1, "");
        let p_root3 = polinomial_new_c(HASH_SIZE, 1, "");

        let pol_bits = self.stark_info.stark_struct.steps[self.stark_info.stark_struct.steps.len() - 1].n_bits;
        let n_trees = self.stark_info.stark_struct.steps.len() as u64;
        let eval_size = self.stark_info.ev_map.len() as u64;

        let p_fri_proof =
            fri_proof_new_c(1 << pol_bits, FIELD_EXTENSION, n_trees, eval_size, self.stark_info.n_publics);

        // let p_input = unsafe { self.verkey.const_root.as_ptr().add(1) } as *mut std::os::raw::c_void;
        let p_input = self.verkey.const_root.as_ptr() as *mut std::os::raw::c_void;
        transcript_put_c(p_transcript, p_input, 4);
        let p_input = proof_ctx.public_inputs.as_ptr() as *mut std::os::raw::c_void;
        transcript_put_c(p_transcript, p_input, self.stark_info.n_publics);

        let p_params = steps_params_new_c(
            self.p_starks,
            p_challenges,
            p_evals,
            p_x_div_x_sub_xi,
            p_x_div_x_sub_wxi,
            proof_ctx.public_inputs.as_ptr() as *mut std::os::raw::c_void,
        );

        timer_stop_and_log!(STARK_INITIALIZATION);

        //--------------------------------
        // 1.- Calculate p_cm1_2ns
        //--------------------------------
        timer_start!(STARK_STEP_1);

        // 1.1 LDE
        timer_start!(STARK_STEP_1_LDE);
        extend_pol_c(self.p_starks, 1);
        timer_stop_and_log!(STARK_STEP_1_LDE);

        // 1.2 MerkleTree
        timer_start!(STARK_STEP_1_MERKLETREE);
        tree_merkelize_c(self.p_starks, 0);
        let p_root0_address = polinomial_get_address_c(p_root0);
        tree_get_root_c(self.p_starks, 0, p_root0_address);
        timer_stop_and_log!(STARK_STEP_1_MERKLETREE);

        info!("Root0: {:?}", unsafe { *(p_root0_address as *mut Goldilocks) });

        transcript_put_c(p_transcript, p_root0_address, HASH_SIZE);
        timer_stop_and_log!(STARK_STEP_1);

        //--------------------------------
        // 2.- Calculate plookups h1 and h2
        //--------------------------------
        timer_start!(STARK_STEP_2);
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 0)); // u
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 1)); // defVal
        if n_rows_step_batch == 4 {
            timer_start!(STARK_STEP_2_CALCULATE_EXPS_AVX);
            step2prev_parser_first_avx_c(self.p_steps, p_params, n, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_2_CALCULATE_EXPS_AVX);
        } else if n_rows_step_batch == 8 {
            timer_start!(STARK_STEP_2_CALCULATE_EXPS_AVX512);
            step2prev_parser_first_avx512_c(self.p_steps, p_params, n, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_2_CALCULATE_EXPS_AVX512);
        } else {
            timer_start!(STARK_STEP_2_CALCULATE_EXPS);
            step2prev_first_parallel_c(self.p_steps, p_params, n);
            timer_stop_and_log!(STARK_STEP_2_CALCULATE_EXPS);
        }

        timer_start!(STARK_STEP_2_CALCULATEH1H2_TRANSPOSE);
        let p_trans_pols = transpose_h1_h2_columns_c(self.p_starks, self.ptr, &num_commited as *const u64, p_buffer);
        timer_stop_and_log!(STARK_STEP_2_CALCULATEH1H2_TRANSPOSE);
        timer_start!(STARK_STEP_2_CALCULATEH1H2);
        calculate_h1_h2_c(self.p_starks, p_trans_pols);
        timer_stop_and_log!(STARK_STEP_2_CALCULATEH1H2);

        timer_start!(STARK_STEP_2_CALCULATEH1H2_TRANSPOSE_2);
        transpose_h1_h2_rows_c(self.p_starks, self.ptr, &num_commited as *const u64, p_trans_pols);
        timer_stop_and_log!(STARK_STEP_2_CALCULATEH1H2_TRANSPOSE_2);

        timer_start!(STARK_STEP_2_LDE);
        extend_pol_c(self.p_starks, 2);
        timer_stop_and_log!(STARK_STEP_2_LDE);

        timer_start!(STARK_STEP_2_MERKLETREE);
        tree_merkelize_c(self.p_starks, 1);
        let p_root1_address = polinomial_get_address_c(p_root1);
        tree_get_root_c(self.p_starks, 1, p_root1_address);
        timer_stop_and_log!(STARK_STEP_2_MERKLETREE);

        info!("MerkleTree rootGL 1: {:?}", unsafe { *(p_root1_address as *mut Goldilocks) });

        transcript_put_c(p_transcript, p_root1_address, HASH_SIZE);
        timer_stop_and_log!(STARK_STEP_2);

        //--------------------------------
        // 3.- Compute Z polynomials
        //--------------------------------
        timer_start!(STARK_STEP_3);
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 2)); // gamma
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 3)); // beta

        if n_rows_step_batch == 4 {
            timer_start!(STARK_STEP_3_CALCULATE_EXPS_AVX);
            step3prev_parser_first_avx_c(self.p_steps, p_params, n, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_3_CALCULATE_EXPS_AVX);
        } else if n_rows_step_batch == 8 {
            timer_start!(STARK_STEP_3_CALCULATE_EXPS_AVX512);
            step3prev_parser_first_avx512_c(self.p_steps, p_params, n, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_3_CALCULATE_EXPS_AVX512);
        } else {
            timer_start!(STARK_STEP_3_CALCULATE_EXPS);
            step3prev_first_parallel_c(self.p_steps, p_params, n);
            timer_stop_and_log!(STARK_STEP_3_CALCULATE_EXPS);
        }

        timer_start!(STARK_STEP_3_CALCULATE_Z_TRANSPOSE);
        let p_newpols = transpose_z_columns_c(self.p_starks, self.ptr, &num_commited as *const u64, p_buffer);
        timer_stop_and_log!(STARK_STEP_3_CALCULATE_Z_TRANSPOSE);

        timer_start!(STARK_STEP_3_CALCULATE_Z);
        calculate_z_c(self.p_starks, p_newpols);
        timer_stop_and_log!(STARK_STEP_3_CALCULATE_Z);

        timer_start!(STARK_STEP_3_CALCULATE_Z_TRANSPOSE_2);
        transpose_z_rows_c(self.p_starks, self.ptr, &num_commited as *const u64, p_newpols);
        timer_stop_and_log!(STARK_STEP_3_CALCULATE_Z_TRANSPOSE_2);

        if n_rows_step_batch == 4 {
            timer_start!(STARK_STEP_3_CALCULATE_EXPS_2_AVX);
            step3_parser_first_avx_c(self.p_steps, p_params, n, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_3_CALCULATE_EXPS_2_AVX);
        } else if n_rows_step_batch == 8 {
            timer_start!(STARK_STEP_3_CALCULATE_EXPS_2_AVX512);
            step3_parser_first_avx512_c(self.p_steps, p_params, n, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_3_CALCULATE_EXPS_2_AVX512);
        } else {
            timer_start!(STARK_STEP_3_CALCULATE_EXPS_2);
            step3_first_parallel_c(self.p_steps, p_params, n);
            timer_stop_and_log!(STARK_STEP_3_CALCULATE_EXPS_2);
        }

        timer_start!(STARK_STEP_3_LDE);
        extend_pol_c(self.p_starks, 3);
        timer_stop_and_log!(STARK_STEP_3_LDE);

        timer_start!(STARK_STEP_3_MERKLETREE);
        tree_merkelize_c(self.p_starks, 2);
        let p_root2_address = polinomial_get_address_c(p_root2);
        tree_get_root_c(self.p_starks, 2, p_root2_address);
        timer_stop_and_log!(STARK_STEP_3_MERKLETREE);

        info!("MerkleTree rootGL 2: {:?}", unsafe { *(p_root2_address as *mut Goldilocks) });

        transcript_put_c(p_transcript, p_root2_address, HASH_SIZE);
        timer_stop_and_log!(STARK_STEP_3);

        //--------------------------------
        // 4. Compute C Polynomial
        //--------------------------------
        timer_start!(STARK_STEP_4);
        timer_start!(STARK_STEP_4_INIT);

        let p_qq1 = polinomial_new_c(n_extended, self.stark_info.q_dim, "qq1");
        let p_qq2 = polinomial_new_c(n_extended * self.stark_info.q_deg, self.stark_info.q_dim, "qq2");
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 4)); // gamma

        let extend_bits = self.stark_info.stark_struct.n_bits_ext - self.stark_info.stark_struct.n_bits;
        timer_stop_and_log!(STARK_STEP_4_INIT);
        if n_rows_step_batch == 4 {
            timer_start!(STARK_STEP_4_CALCULATE_EXPS_2NS_AVX);
            step42ns_parser_first_avx_c(self.p_steps, p_params, n_extended, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_4_CALCULATE_EXPS_2NS_AVX);
        } else if n_rows_step_batch == 8 {
            timer_start!(STARK_STEP_4_CALCULATE_EXPS_2NS_AVX512);
            step42ns_parser_first_avx512_c(self.p_steps, p_params, n_extended, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_4_CALCULATE_EXPS_2NS_AVX512);
        } else {
            timer_start!(STARK_STEP_4_CALCULATE_EXPS_2NS);
            step42ns_first_parallel_c(self.p_steps, p_params, n_extended);
            timer_stop_and_log!(STARK_STEP_4_CALCULATE_EXPS_2NS);
        }

        calculate_exps_2ns_c(self.p_starks, p_qq1, p_qq2);

        timer_start!(STARK_STEP_4_MERKLETREE);
        tree_merkelize_c(self.p_starks, 3);
        let p_root3_address = polinomial_get_address_c(p_root3);
        tree_get_root_c(self.p_starks, 3, p_root3_address);
        info!("MerkleTree rootGL 3: {:?}", unsafe { *(p_root3_address as *mut Goldilocks) });
        timer_stop_and_log!(STARK_STEP_4_MERKLETREE);

        transcript_put_c(p_transcript, p_root3_address, HASH_SIZE);
        timer_stop_and_log!(STARK_STEP_4);

        //--------------------------------
        // 5. Compute FRI Polynomial
        //--------------------------------
        timer_start!(STARK_STEP_5);

        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 7)); // xi

        timer_start!(STARK_STEP_5_LEv_LpEv);
        let p_l_ev = polinomial_new_c(n, FIELD_EXTENSION, "LEv");
        let p_lp_ev = polinomial_new_c(n, FIELD_EXTENSION, "LpEv");
        let p_xis = polinomial_new_c(1, FIELD_EXTENSION, "");
        let p_wxis = polinomial_new_c(1, FIELD_EXTENSION, "");
        let p_c_w = polinomial_new_c(1, FIELD_EXTENSION, "");

        calculate_lev_lpev_c(self.p_starks, p_l_ev, p_lp_ev, p_xis, p_wxis, p_c_w, p_challenges);
        timer_stop_and_log!(STARK_STEP_5_LEv_LpEv);

        timer_start!(STARK_STEP_5_EVMAP);
        evmap_c(self.p_starks, self.ptr, p_evals, p_l_ev, p_lp_ev);
        timer_stop_and_log!(STARK_STEP_5_EVMAP);

        for i in 0..self.stark_info.ev_map.len() {
            let p_evals_i = polinomial_get_p_element_c(p_evals, i as u64);
            transcript_put_c(p_transcript, p_evals_i, 3);
        }
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 5)); // v1
        transcript_get_field_c(p_transcript, polinomial_get_p_element_c(p_challenges, 6)); // v2

        // Calculate xDivXSubXi, xDivXSubWXi
        timer_start!(STARK_STEP_5_XDIVXSUB);
        let p_xi = polinomial_new_c(1, FIELD_EXTENSION, "");
        let p_wxi = polinomial_new_c(1, FIELD_EXTENSION, "");

        calculate_xdivxsubxi_c(
            self.p_starks,
            extend_bits,
            p_xi,
            p_wxi,
            p_challenges,
            p_x_div_x_sub_xi,
            p_x_div_x_sub_wxi,
        );
        timer_stop_and_log!(STARK_STEP_5_XDIVXSUB);

        if n_rows_step_batch == 4 {
            timer_start!(STARK_STEP_5_CALCULATE_EXPS_AVX);
            step52ns_parser_first_avx_c(self.p_steps, p_params, n_extended, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_5_CALCULATE_EXPS_AVX);
        } else if n_rows_step_batch == 8 {
            timer_start!(STARK_STEP_5_CALCULATE_EXPS_AVX512);
            step52ns_parser_first_avx512_c(self.p_steps, p_params, n_extended, n_rows_step_batch);
            timer_stop_and_log!(STARK_STEP_5_CALCULATE_EXPS_AVX512);
        } else {
            timer_start!(STARK_STEP_5_CALCULATE_EXPS);
            step52ns_first_parallel_c(self.p_steps, p_params, n_extended);
            timer_stop_and_log!(STARK_STEP_5_CALCULATE_EXPS);
        }
        timer_stop_and_log!(STARK_STEP_5);

        timer_start!(STARK_STEP_FRI);
        finalize_proof_c(self.p_starks, p_fri_proof, p_transcript, p_evals, p_root0, p_root1, p_root2, p_root3);
        timer_stop_and_log!(STARK_STEP_FRI);

        timer_stop_and_log!(STARKS_COMPUTE_STAGE);

        proof_ctx.proof = Some(p_fri_proof);
        info!("{}: <-- eStark prover - STAGE {}", Self::MY_NAME, stage_id);

        return;
    }
}
