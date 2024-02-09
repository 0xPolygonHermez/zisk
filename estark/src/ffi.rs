#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use goldilocks::Goldilocks;
use super::verification_key::VerificationKey;

use ::std::os::raw::c_void;

#[cfg(not(feature = "no_lib_link"))]
include!("../bindings.rs");

#[cfg(not(feature = "no_lib_link"))]
use std::ffi::CString;

#[cfg(not(feature = "no_lib_link"))]
pub fn zkevm_main_c(config_filename: &str, ptr: *mut u8) {
    unsafe {
        let config_filename = CString::new(config_filename).unwrap();

        zkevm_main(config_filename.as_ptr() as *mut std::os::raw::c_char, ptr as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn zkevm_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { zkevm_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn zkevm_steps_free_c(pZkevmSteps: *mut c_void) {
    unsafe {
        zkevm_steps_free(pZkevmSteps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn c12a_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { c12a_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn c12a_steps_free_c(pC12aSteps: *mut c_void) {
    unsafe {
        c12a_steps_free(pC12aSteps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn recursive1_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { recursive1_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn recursive1_steps_free_c(pRecursive1Steps: *mut c_void) {
    unsafe {
        recursive1_steps_free(pRecursive1Steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn _recursive2_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { recursive2_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn _recursive2_steps_free_c(pRecursive2Steps: *mut c_void) {
    unsafe {
        recursive2_steps_free(pRecursive2Steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step2prev_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step2prev_parser_first_avx(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step2prev_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step2prev_parser_first_avx512(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step2prev_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    unsafe {
        step2prev_first_parallel(_pSteps, _pParams, _nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3prev_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step3prev_parser_first_avx(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3prev_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step3prev_parser_first_avx512(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3prev_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    unsafe {
        step3prev_first_parallel(_pSteps, _pParams, _nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step3_parser_first_avx(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step3_parser_first_avx512(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    unsafe {
        step3_first_parallel(_pSteps, _pParams, _nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step42ns_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step42ns_parser_first_avx(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step42ns_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step42ns_parser_first_avx512(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step42ns_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    unsafe {
        step42ns_first_parallel(_pSteps, _pParams, _nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step52ns_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step52ns_parser_first_avx(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step52ns_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    unsafe {
        step52ns_parser_first_avx512(_pSteps, _pParams, _nrows, _nrowsBatch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step52ns_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    unsafe {
        step52ns_first_parallel(_pSteps, _pParams, _nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_new_c(
    pol_n: u64,
    dim: u64,
    num_trees: u64,
    eval_size: u64,
    n_publics: u64,
) -> *mut std::os::raw::c_void {
    unsafe { fri_proof_new(pol_n, dim, num_trees, eval_size, n_publics) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_free_c(pZkevmSteps: *mut c_void) {
    unsafe {
        fri_proof_free(pZkevmSteps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn config_new_c(config_filename: &str) -> *mut std::os::raw::c_void {
    unsafe {
        let config_filename = CString::new(config_filename).unwrap();

        config_new(config_filename.as_ptr() as *mut std::os::raw::c_char)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn config_free_c(pConfig: *mut c_void) {
    unsafe {
        config_free(pConfig);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_new_c(
    p_config: *mut c_void,
    const_pols: &str,
    map_const_pols_file: bool,
    constants_tree: &str,
    stark_info: &str,
    p_address: *mut c_void,
) -> *mut c_void {
    unsafe {
        let const_pols = CString::new(const_pols).unwrap();
        let constants_tree = CString::new(constants_tree).unwrap();
        let stark_info = CString::new(stark_info).unwrap();

        starks_new(
            p_config,
            const_pols.as_ptr() as *mut std::os::raw::c_char,
            map_const_pols_file,
            constants_tree.as_ptr() as *mut std::os::raw::c_char,
            stark_info.as_ptr() as *mut std::os::raw::c_char,
            p_address,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_genproof_c<T>(
    _p_Starks: *mut c_void,
    _p_fri_proof: *mut c_void,
    _p_public_inputs: &Vec<T>,
    _p_verkey: &VerificationKey<Goldilocks>,
    _p_steps: *mut c_void,
) {
    unsafe {
        starks_genproof(
            _p_Starks,
            _p_fri_proof,
            _p_public_inputs.as_ptr() as *mut std::os::raw::c_void,
            _p_verkey.const_root.as_ptr() as *mut std::os::raw::c_void,
            _p_steps,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_free_c(p_starks: *mut c_void) {
    unsafe {
        starks_free(p_starks);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transpose_h1_h2_columns_c(
    p_starks: *mut c_void,
    p_address: *mut c_void,
    num_commited: *const u64,
    p_buffer: *mut c_void,
) -> *mut c_void {
    unsafe { transpose_h1_h2_columns(p_starks, p_address, num_commited, p_buffer) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transpose_h1_h2_rows_c(
    p_starks: *mut c_void,
    p_address: *mut c_void,
    num_commited: *const u64,
    p_trans_pols: *mut c_void,
) {
    unsafe {
        transpose_h1_h2_rows(p_starks, p_address, num_commited, p_trans_pols);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transpose_z_columns_c(
    p_starks: *mut c_void,
    p_address: *mut c_void,
    num_commited: *const u64,
    p_buffer: *mut c_void,
) -> *mut c_void {
    unsafe { transpose_z_columns(p_starks, p_address, num_commited, p_buffer) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transpose_z_rows_c(
    p_starks: *mut c_void,
    p_address: *mut c_void,
    num_commited: *const u64,
    p_trans_pols: *mut c_void,
) {
    unsafe {
        transpose_z_rows(p_starks, p_address, num_commited, p_trans_pols);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn evmap_c(
    p_starks: *mut c_void,
    p_address: *mut c_void,
    evals: *mut c_void,
    p_l_ev: *mut c_void,
    p_lp_ev: *mut c_void,
) {
    unsafe {
        evmap(p_starks, p_address, evals, p_l_ev, p_lp_ev);
    }
}

// #[cfg(not(feature = "no_lib_link"))]
// pub fn transcript_add_array_c<T>(
//     p_starks: *mut c_void,
//     p_transcript: *mut c_void,
//     elements: *const T,
//     n_elements: u64,
// ) {
//     unsafe {
//         transcript_add_array(p_starks, p_transcript, elements as *mut std::os::raw::c_void, n_elemenets as u64);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn transcript_add_polynomial_c(
//     p_starks: *mut c_void,
//     pt_ranscript: *mut c_void,
//     p_polynomial: *mut c_void,
// ) {
//     unsafe {
//         transcript_add_polynomial(p_starks, pt_ranscript, p_polynomial);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]

// pub fn get_challenges_c(
//     p_transcript: *mut c_void,
//     p_steps_params: *mut c_void,
//     n_challenges: u64,
//     index: u64,
// ) {
//     unsafe {
//         get_challenges(p_transcript, p_steps_params, n_challenges, index);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]

// pub fn step_params_new_c(p_starks: *mut c_void) -> *mut c_void {
//     unsafe { steps_params_new(p_starks) }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn steps_params_free_c(p_steps_params: *mut c_void) {
//     unsafe {
//         steps_params_free(p_steps_params);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn extend_and_merkelize_c(
//     p_starks: *mut c_void,
//     step: u64,
//     p_steps_params: *mut c_void,
//     p_stark_info: *mut c_void,
//     p_proof: *mut c_void,
// ) {
//     unsafe {
//         extend_and_merkelize(p_starks, step, p_steps_params, p_stark_info, p_proof);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_expressions_c(
//     p_starks: *mut c_void,
//     step: &str,
//     n_rows_step_batch: u64,
//     p_steps: *mut c_void,
//     p_steps_params: *mut c_void,
//     N: u64,
// ) {
//     let step = CString::new(step).unwrap();

//     unsafe {
//         calculate_expressions(p_starks, step, n_rows_step_batch, p_steps, p_steps_params, N);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn get_stark_info_c(p_starks: *mut c_void) -> *mut c_void {
//     unsafe { get_stark_info(p_starks) }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn get_proof_c(p_starks: *mut c_void) -> *mut c_void {
//     unsafe { get_proof(p_starks) }
// }

#[cfg(not(feature = "no_lib_link"))]
pub fn steps_params_new_c(
    p_starks: *mut c_void,
    p_challenges: *mut c_void,
    p_evals: *mut c_void,
    p_x_div_x_sub_xi: *mut c_void,
    p_x_div_x_sub_wxi: *mut c_void,
    p_public_inputs: *mut c_void,
) -> *mut c_void {
    unsafe { steps_params_new(p_starks, p_challenges, p_evals, p_x_div_x_sub_xi, p_x_div_x_sub_wxi, p_public_inputs) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn steps_params_free_c(p_steps_params: *mut c_void) {
    unsafe {
        steps_params_free(p_steps_params);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn tree_merkelize_c(p_starks: *mut c_void, index: u64) {
    unsafe {
        tree_merkelize(p_starks, index);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn tree_get_root_c(p_starks: *mut c_void, index: u64, root: *mut c_void) {
    unsafe {
        tree_get_root(p_starks, index, root);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn extend_pol_c(p_starks: *mut c_void, step: u64) {
    unsafe {
        extend_pol(p_starks, step);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_pbuffer_c(p_starks: *mut c_void) -> *mut c_void {
    unsafe { get_pbuffer(p_starks) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_h1_h2_c(p_starks: *mut c_void, p_trans_pols: *mut c_void) {
    unsafe {
        calculate_h1_h2(p_starks, p_trans_pols);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_z_c(p_starks: *mut c_void, p_new_pols: *mut c_void) {
    unsafe {
        calculate_z(p_starks, p_new_pols);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_exps_2ns_c(p_starks: *mut c_void, p_qq1: *mut c_void, p_qq2: *mut c_void) {
    unsafe {
        calculate_exps_2ns(p_starks, p_qq1, p_qq2);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_lev_lpev_c(
    p_starks: *mut c_void,
    p_l_ev: *mut c_void,
    p_lp_ev: *mut c_void,
    p_xis: *mut c_void,
    p_wxis: *mut c_void,
    p_c_w: *mut c_void,
    p_challenges: *mut c_void,
) {
    unsafe {
        calculate_lev_lpev(p_starks, p_l_ev, p_lp_ev, p_xis, p_wxis, p_c_w, p_challenges);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_xdivxsubxi_c(
    p_starks: *mut c_void,
    extend_bits: u64,
    xi: *mut c_void,
    wxi: *mut c_void,
    challenges: *mut c_void,
    p_x_div_x_sub_xi: *mut c_void,
    p_x_div_x_sub_wxi: *mut c_void,
) {
    unsafe {
        calculate_xdivxsubxi(p_starks, extend_bits, xi, wxi, challenges, p_x_div_x_sub_xi, p_x_div_x_sub_wxi);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn finalize_proof_c(
    p_starks: *mut c_void,
    p_proof: *mut c_void,
    p_transcript: *mut c_void,
    p_evals: *mut c_void,
    p_root0: *mut c_void,
    p_root1: *mut c_void,
    p_root2: *mut c_void,
    p_root3: *mut c_void,
) {
    unsafe {
        finalize_proof(p_starks, p_proof, p_transcript, p_evals, p_root0, p_root1, p_root2, p_root3);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_num_rows_step_batch_c(pStarks: *mut c_void) -> u64 {
    unsafe { get_num_rows_step_batch(pStarks) }
}

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_h1h2_c(
//     p_starks: *mut c_void,
//     p_steps_params: *mut c_void,
//     p_stark_info: *mut c_void,
// ) {
//     unsafe {
//         calculate_h1h2(p_starks, p_steps_params, p_stark_info);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_z_c(
//     p_starks: *mut c_void,
//     p_steps_params: *mut c_void,
//     p_stark_info: *mut c_void,
// ) {
//     unsafe {
//         calculate_z(p_starks, p_steps_params, p_stark_info);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_q_c(
//     p_starks: *mut c_void,
//     p_steps_params: *mut c_void,
//     p_stark_info: *mut c_void,
//     p_proof: *mut c_void,
// ) {
//     unsafe {
//         calculate_q(p_starks, p_steps_params, p_stark_info, p_proof);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_evals_c(
//     p_starks: *mut c_void,
//     p_steps_params: *mut c_void,
//     p_stark_info: *mut c_void,
//     p_proof: *mut c_void,
// ) {
//     unsafe {
//         calculate_evals(p_starks, p_steps_params, p_stark_info, p_proof);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_fri_pol_c(
//     p_starks: *mut c_void,
//     p_steps_params: *mut c_void,
//     p_stark_info: *mut c_void,
//     p_steps: *mut c_void,
//     n_rows_step_batch: u64,
// ) -> *mut c_void {
//     unsafe { calculate_fri_pol(p_starks, p_steps_params, p_stark_info, p_steps, n_rows_step_batch) }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_fri_folding_c(
//     p_starks: *mut c_void,
//     p_stark_info: *mut c_void,
//     p_proof: *mut c_void,
//     p_fri_pol: *mut c_void,
//     step: u64,
//     p_polinomial: *mut c_void,
// ) {
//     unsafe {
//         calculate_fri_folding(p_starks, p_stark_info, p_proof, p_fri_pol, step, p_polinomial);
//     }
// }

// #[cfg(not(feature = "no_lib_link"))]
// pub fn calculate_fri_queries_c(
//     p_starks: *mut c_void,
//     p_stark_info: *mut c_void,
//     p_proof: *mut c_void,
//     p_fri_pol: *mut c_void,
//     fri_queries: Vec<u64>,
// ) {
//     unsafe {
//         calculate_fri_queries(
//             p_starks,
//             p_stark_info,
//             p_proof,
//             p_fri_pol,
//             fri_queries.as_ptr() as *mut u64,
//             fri_queries.len() as u64,
//         );
//     }
// }

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_pols_starks_new_c(pAddress: *mut c_void, degree: u64, nCommitedPols: u64) -> *mut std::os::raw::c_void {
    unsafe { commit_pols_starks_new(pAddress, degree, nCommitedPols) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_pols_starks_free_c(pCommitPolsStarks: *mut c_void) {
    unsafe {
        commit_pols_starks_free(pCommitPolsStarks);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn circom_get_commited_pols_c(
    p_commit_pols_starks: *mut c_void,
    zkevm_ferifier: &str,
    exec_file: &str,
    zkin: *mut c_void,
    n: u64,
    n_cols: u64,
) {
    unsafe {
        let zkevm_ferifier = CString::new(zkevm_ferifier).unwrap();
        let exec_file = CString::new(exec_file).unwrap();

        circom_get_commited_pols(
            p_commit_pols_starks,
            zkevm_ferifier.as_ptr() as *mut std::os::raw::c_char,
            exec_file.as_ptr() as *mut std::os::raw::c_char,
            zkin,
            n,
            n_cols,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn circom_recursive1_get_commited_pols_c(
    p_commit_pols_starks: *mut c_void,
    zkevm_ferifier: &str,
    exec_file: &str,
    zkin: *mut c_void,
    n: u64,
    n_cols: u64,
) {
    unsafe {
        let zkevm_ferifier = CString::new(zkevm_ferifier).unwrap();
        let exec_file = CString::new(exec_file).unwrap();

        circom_recursive1_get_commited_pols(
            p_commit_pols_starks,
            zkevm_ferifier.as_ptr() as *mut std::os::raw::c_char,
            exec_file.as_ptr() as *mut std::os::raw::c_char,
            zkin,
            n,
            n_cols,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn zkin_new_c<T>(p_fri_proof: *mut c_void, public_inputs: &Vec<T>, root_c: &Vec<Goldilocks>) -> *mut c_void {
    unsafe {
        zkin_new(
            p_fri_proof,
            public_inputs.len() as std::os::raw::c_ulong,
            public_inputs.as_ptr() as *mut std::os::raw::c_void,
            root_c.len() as std::os::raw::c_ulong,
            root_c.as_ptr() as *mut std::os::raw::c_void,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_new_c() -> *mut c_void {
    unsafe { transcript_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_put_c(_pTranscript: *mut c_void, _pInput: *mut c_void, _size: u64) {
    unsafe {
        transcript_put(_pTranscript, _pInput, _size);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_get_field_c(_pTranscript: *mut c_void, _pOutput: *mut c_void) {
    unsafe {
        transcript_get_field(_pTranscript, _pOutput);
    }
}

// #[cfg(not(feature = "no_lib_link"))]
// pub fn get_permutations_c(p_transcript: *mut c_void, res: &[u64], n: u64, nBits: u64) {
//     unsafe {
//         get_permutations(p_transcript, res.as_ptr() as *mut u64, n, nBits);
//     }
// }

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_free_c(p_transcript: *mut c_void) {
    unsafe {
        transcript_free(p_transcript);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_new_c(_degree: u64, _dim: u64, _name: &str) -> *mut c_void {
    unsafe {
        let name = CString::new(_name).unwrap();

        polinomial_new(_degree, _dim, name.as_ptr() as *mut std::os::raw::c_char)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_get_address_c(_pPolinomial: *mut c_void) -> *mut c_void {
    unsafe { polinomial_get_address(_pPolinomial) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_get_p_element_c(_pPolinomial: *mut c_void, _index: u64) -> *mut c_void {
    unsafe { polinomial_get_p_element(_pPolinomial, _index) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_free_c(_pPolinomial: *mut c_void) {
    unsafe {
        polinomial_free(_pPolinomial);
    }
}

// MOCK METHODS FOR TESTING

#[cfg(feature = "no_lib_link")]
pub fn zkevm_main_c(config_filename: &str, _ptr: *mut u8) {
    println!("zkevm_main_c: This is a mock call because there is no linked library {}", config_filename);
}

#[cfg(feature = "no_lib_link")]
pub fn zkevm_steps_new_c() -> *mut std::os::raw::c_void {
    println!("zkevm_steps_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn zkevm_steps_free_c(_pZkevmSteps: *mut c_void) {
    println!("zkevm_steps_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn c12a_steps_new_c() -> *mut std::os::raw::c_void {
    println!("c12a_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn c12a_steps_free_c(_pC12aSteps: *mut c_void) {
    println!("c12a_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn recursive1_steps_new_c() -> *mut std::os::raw::c_void {
    println!("recursive1_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn recursive1_steps_free_c(_pRecursive1Steps: *mut c_void) {
    println!("recursive1_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn _recursive2_steps_new_c() -> *mut std::os::raw::c_void {
    println!("recursive2_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn _recursive2_steps_free_c(_pRecursive2Steps: *mut c_void) {
    println!("recursive2_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step2prev_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step2prev_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step2prev_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step2prev_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step2prev_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    println!("step2prev_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3prev_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step3prev_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3prev_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step3prev_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3prev_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    println!("step3prev_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step3_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step3_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    println!("step3_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step42ns_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step42ns_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step42ns_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step42ns_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step42ns_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    println!("step42ns_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step52ns_parser_first_avx_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step52ns_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step52ns_parser_first_avx512_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64, _nrowsBatch: u64) {
    println!("step52ns_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step52ns_first_parallel_c(_pSteps: *mut c_void, _pParams: *mut c_void, _nrows: u64) {
    println!("step52ns_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_new_c(
    _polN: u64,
    _dim: u64,
    _numTrees: u64,
    _evalSize: u64,
    _nPublics: u64,
) -> *mut std::os::raw::c_void {
    println!("fri_proof_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_free_c(_pZkevmSteps: *mut c_void) {
    println!("fri_proof_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn config_new_c(config_filename: &str) -> *mut std::os::raw::c_void {
    println!("config_new: This is a mock call because there is no linked library {}", config_filename);
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn config_free_c(_pConfig: *mut c_void) {
    println!("config_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn starks_new_c(
    _p_config: *mut c_void,
    _const_pols: &str,
    _map_const_pols_file: bool,
    _constants_tree: &str,
    _stark_info: &str,
    _p_address: *mut c_void,
) -> *mut c_void {
    println!("starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn starks_genproof_c<T>(
    _p_Starks: *mut c_void,
    _p_fri_proof: *mut c_void,
    _p_public_inputs: &Vec<T>,
    _p_verkey: &VerificationKey<Goldilocks>,
    _p_steps: *mut c_void,
) {
    println!("starks_genproof_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn starks_free_c(_pStarks: *mut c_void) {
    println!("starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transpose_h1_h2_columns_c(
    _pStarks: *mut c_void,
    _pAddress: *mut c_void,
    _numCommited: *const u64,
    _pBuffer: *mut c_void,
) -> *mut c_void {
    println!("transpose_h1_h2_columns: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transpose_h1_h2_rows_c(
    _pStarks: *mut c_void,
    _pAddress: *mut c_void,
    _numCommited: *const u64,
    _transPols: *mut c_void,
) {
    println!("transpose_h1_h2_rows: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transpose_z_columns_c(
    _pStarks: *mut c_void,
    _pAddress: *mut c_void,
    _numCommited: *const u64,
    _pBuffer: *mut c_void,
) -> *mut c_void {
    println!("transpose_z_columns: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transpose_z_rows_c(
    _pStarks: *mut c_void,
    _pAddress: *mut c_void,
    _numCommited: *const u64,
    _transPols: *mut c_void,
) {
    println!("transpose_z_rows: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn evmap_c(
    _pStarks: *mut c_void,
    _pAddress: *mut c_void,
    _evals: *mut c_void,
    _LEv: *mut c_void,
    _LpEv: *mut c_void,
) {
    println!("evmap: This is a mock call because there is no linked library");
}

// #[cfg(feature = "no_lib_link")]
// pub fn transcript_add_array_c<T>(
//     _p_starks: *mut c_void,
//     _p_transcript: *mut c_void,
//     _elements: *const T,
//     _n_elements: u64,
// ) {
//     println!("transcript_add_array: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn transcript_add_polynomial_c(
//     _p_starks: *mut c_void,
//     _p_transcript: *mut c_void,
//     _p_polynomial: *mut c_void,
// ) {
//     println!("transcript_add_polynomial: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn get_challenges_c(
//     _p_transcript: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _n_challenges: u64,
//     _index: u64,
// ) {
//     println!("get_challenges: This is a mock call because there is no linked library");
// }

#[cfg(feature = "no_lib_link")]
pub fn steps_params_new_c(
    _pStarks: *mut c_void,
    _pChallenges: *mut c_void,
    _pEvals: *mut c_void,
    _pXDivXSubXi: *mut c_void,
    _pXDivXSubWXi: *mut c_void,
    _pPublicInputs: *mut c_void,
) -> *mut c_void {
    println!("steps_params_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn steps_params_free_c(_pStepsParams: *mut c_void) {
    println!("steps_params_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn tree_merkelize_c(_pStarks: *mut c_void, _index: u64) {
    println!("tree_merkelize: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn tree_get_root_c(_pStarks: *mut c_void, _index: u64, _root: *mut c_void) {
    println!("tree_get_root: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn extend_pol_c(_pStarks: *mut c_void, _step: u64) {
    println!("extend_pol: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_pbuffer_c(_pStarks: *mut c_void) -> *mut c_void {
    println!("get_pbuffer: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_h1_h2_c(_pStarks: *mut c_void, _pTransPols: *mut c_void) {
    println!("calculate_h1_h2: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_z_c(_pStarks: *mut c_void, _pNewPols: *mut c_void) {
    println!("calculate_z: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_exps_2ns_c(_pStarks: *mut c_void, _pQq1: *mut c_void, _pQq2: *mut c_void) {
    println!("calculate_exps_2ns_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_lev_lpev_c(
    _pStarks: *mut c_void,
    _pLEv: *mut c_void,
    _pLpEv: *mut c_void,
    _pXis: *mut c_void,
    _pWxis: *mut c_void,
    _pC_w: *mut c_void,
    _pChallenges: *mut c_void,
) {
    println!("calculate_lev_lpev: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_xdivxsubxi_c(
    _pStarks: *mut c_void,
    _extendBits: u64,
    _xi: *mut c_void,
    _wxi: *mut c_void,
    _challenges: *mut c_void,
    _xDivXSubXi: *mut c_void,
    _xDivXSubWXi: *mut c_void,
) {
    println!("calculate_xdivxsubxi: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn finalize_proof_c(
    _pStarks: *mut c_void,
    _pProof: *mut c_void,
    _transcript: *mut c_void,
    _evals: *mut c_void,
    _root0: *mut c_void,
    _root1: *mut c_void,
    _root2: *mut c_void,
    _root3: *mut c_void,
) {
    println!("finalize_proof: This is a mock call because there is no linked library");
}

// #[cfg(feature = "no_lib_link")]
// pub fn extend_and_merkelize_c(
//     _p_starks: *mut c_void,
//     _step: u64,
//     _p_steps_params: *mut c_void,
//     _p_stark_info: *mut c_void,
//     _p_proof: *mut c_void,
// ) {
//     println!("extend_and_merkelize: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_expressions_c(
//     _p_starks: *mut c_void,
//     _step: &str,
//     _n_rows_step_batch: u64,
//     _p_steps: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _N: u64,
// ) {
//     println!("calculate_expressions: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn get_stark_info_c(_p_starks: *mut c_void) -> *mut c_void {
//     println!("get_stark_info: This is a mock call because there is no linked library");
//     std::ptr::null_mut()
// }

// #[cfg(feature = "no_lib_link")]
// pub fn get_proof_c(_p_starks: *mut c_void) -> *mut c_void {
//     println!("get_proof: This is a mock call because there is no linked library");
//     std::ptr::null_mut()
// }

#[cfg(feature = "no_lib_link")]
pub fn get_num_rows_step_batch_c(_pStarks: *mut c_void) -> u64 {
    println!("get_num_rows_step_batch: This is a mock call because there is no linked library");
    1
}

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_h1h2_c(
//     _p_starks: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _p_stark_info: *mut c_void,
// ) {
//     println!("calculate_h1h2: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_z_c(
//     _p_starks: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _p_stark_info: *mut c_void,
// ) {
//     println!("calculate_z: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_q_c(
//     _p_starks: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _p_stark_info: *mut c_void,
//     _p_proof: *mut c_void,
// ) {
//     println!("calculate_q: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_evals_c(
//     _p_starks: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _p_stark_info: *mut c_void,
//     _p_proof: *mut c_void,
// ) {
//     println!("calculate_evals: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_fri_pol_c(
//     _p_starks: *mut c_void,
//     _p_steps_params: *mut c_void,
//     _p_stark_info: *mut c_void,
//     _p_steps: *mut c_void,
//     _n_rows_step_batch: u64,
// ) -> *mut c_void {
//     println!("calculate_fri_pol: This is a mock call because there is no linked library");
//     std::ptr::null_mut()
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_fri_folding_c(
//     _p_starks: *mut c_void,
//     _p_stark_info: *mut c_void,
//     _p_proof: *mut c_void,
//     _p_fri_pol: *mut c_void,
//     _step: u64,
//     _p_polinomial: *mut c_void,
// ) {
//     println!("calculate_fri_folding: This is a mock call because there is no linked library");
// }

// #[cfg(feature = "no_lib_link")]
// pub fn calculate_fri_queries_c(
//     _p_starks: *mut c_void,
//     _p_stark_info: *mut c_void,
//     _p_proof: *mut c_void,
//     _p_fri_pol: *mut c_void,
//     _fri_queries: Vec<u64>,
// ) {
//     println!("calculate_fri_queries: This is a mock call because there is no linked library");
// }

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_starks_new_c(
    _pAddress: *mut c_void,
    _degree: u64,
    _nCommitedPols: u64,
) -> *mut std::os::raw::c_void {
    println!("commit_pols_starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_starks_free_c(_pCommitPolsStarks: *mut c_void) {
    println!("commit_pols_starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn circom_get_commited_pols_c(
    _p_commit_pols_starks: *mut c_void,
    _zkevm_ferifier: &str,
    _exec_file: &str,
    _zkin: *mut c_void,
    _n: u64,
    _n_cols: u64,
) {
    println!("circom_get_commited_pols: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn circom_recursive1_get_commited_pols_c(
    _p_commit_pols_starks: *mut c_void,
    _zkevm_ferifier: &str,
    _exec_file: &str,
    _zkin: *mut c_void,
    _n: u64,
    _n_cols: u64,
) {
    println!("circom_recursive1_get_commited_pols: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn zkin_new_c<T>(_p_fri_proof: *mut c_void, _public_inputs: &Vec<T>, _root_c: &Vec<Goldilocks>) -> *mut c_void {
    println!("zkin_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_new_c() -> *mut c_void {
    println!("transcript_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_put_c(_pTranscript: *mut c_void, _pInput: *mut c_void, _size: u64) {
    println!("transcript_put: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_get_field_c(_pTranscript: *mut c_void, _pOutput: *mut c_void) {
    println!("transcript_get_field: This is a mock call because there is no linked library");
}

// #[cfg(feature = "no_lib_link")]
// pub fn get_permutations_c(_p_transcript: *mut c_void, _res: &[u64], _n: u64, _nBits: u64) {
//     println!("get_permutations: This is a mock call because there is no linked library");
// }

#[cfg(feature = "no_lib_link")]
pub fn transcript_free_c(_p_transcript: *mut c_void) {
    println!("transcript_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_new_c(_degree: u64, _dim: u64, _name: &str) -> *mut c_void {
    println!("polinomial_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_get_address_c(_pPolinomial: *mut c_void) -> *mut c_void {
    println!("get_address: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_get_p_element_c(_pPolinomial: *mut c_void, _index: u64) -> *mut c_void {
    println!("get_p_element: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_free_c(_pPolinomial: *mut c_void) {
    println!("polinomial_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_new_c(_pAddress: *mut c_void, _degree: u64) -> *mut c_void {
    println!("commit_pols_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_free_c(_pCommitPols: *mut c_void) {
    println!("commit_pols_free: This is a mock call because there is no linked library");
}
