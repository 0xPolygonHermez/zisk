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
pub fn zkevm_steps_free_c(p_zkevm_steps: *mut c_void) {
    unsafe {
        zkevm_steps_free(p_zkevm_steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn c12a_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { c12a_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn c12a_steps_free_c(p_c12a_steps: *mut c_void) {
    unsafe {
        c12a_steps_free(p_c12a_steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn recursive1_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { recursive1_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn recursive1_steps_free_c(p_recursive1_steps: *mut c_void) {
    unsafe {
        recursive1_steps_free(p_recursive1_steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn _recursive2_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { recursive2_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn _recursive2_steps_free_c(p_recursive2_steps: *mut c_void) {
    unsafe {
        recursive2_steps_free(p_recursive2_steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step2prev_parser_first_avx_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step2prev_parser_first_avx(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step2prev_parser_first_avx512_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step2prev_parser_first_avx512(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step2prev_first_parallel_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64) {
    unsafe {
        step2prev_first_parallel(p_steps, p_params, nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3prev_parser_first_avx_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step3prev_parser_first_avx(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3prev_parser_first_avx512_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step3prev_parser_first_avx512(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3prev_first_parallel_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64) {
    unsafe {
        step3prev_first_parallel(p_steps, p_params, nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3_parser_first_avx_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step3_parser_first_avx(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3_parser_first_avx512_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step3_parser_first_avx512(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step3_first_parallel_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64) {
    unsafe {
        step3_first_parallel(p_steps, p_params, nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step42ns_parser_first_avx_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step42ns_parser_first_avx(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step42ns_parser_first_avx512_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step42ns_parser_first_avx512(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step42ns_first_parallel_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64) {
    unsafe {
        step42ns_first_parallel(p_steps, p_params, nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step52ns_parser_first_avx_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step52ns_parser_first_avx(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step52ns_parser_first_avx512_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64, n_rows_batch: u64) {
    unsafe {
        step52ns_parser_first_avx512(p_steps, p_params, nrows, n_rows_batch);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn step52ns_first_parallel_c(p_steps: *mut c_void, p_params: *mut c_void, nrows: u64) {
    unsafe {
        step52ns_first_parallel(p_steps, p_params, nrows);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_new_c(p_starks: *mut c_void) -> *mut c_void {
    unsafe { fri_proof_new(p_starks) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_get_root_c(pFriProof: *mut c_void, root_index: u64, root_subindex: u64) -> *mut c_void {
    unsafe { fri_proof_get_root(pFriProof, root_index, root_subindex) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_get_tree_root_c(pFriProof: *mut c_void, tree_index: u64, root_index: u64) -> *mut c_void {
    unsafe { fri_proof_get_tree_root(pFriProof, tree_index, root_index) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_free_c(p_zkevm_steps: *mut c_void) {
    unsafe {
        fri_proof_free(p_zkevm_steps);
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
pub fn get_stark_info_c(p_stark: *mut c_void) -> *mut c_void {
    unsafe { get_stark_info(p_stark) }
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
pub fn extend_and_merkelize_c(p_stark: *mut c_void, step: u64, p_params: *mut c_void, proof: *mut c_void) {
    unsafe {
        extend_and_merkelize(p_stark, step, p_params, proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_h1_h2_c(p_starks: *mut c_void, p_params: *mut c_void) {
    unsafe {
        calculate_h1_h2(p_starks, p_params);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_z_c(p_starks: *mut c_void, p_params: *mut c_void) {
    unsafe {
        calculate_z(p_starks, p_params);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_expressions_c(
    p_stark: *mut c_void,
    step: &str,
    nrowsStepBatch: u64,
    p_steps: *mut c_void,
    p_params: *mut c_void,
    n: u64,
) {
    let step = CString::new(step).unwrap();

    unsafe {
        calculate_expressions(
            p_stark,
            step.as_ptr() as *mut std::os::raw::c_char,
            nrowsStepBatch,
            p_steps,
            p_params,
            n,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_q_c(p_stark: *mut c_void, p_params: *mut c_void, pProof: *mut c_void) {
    unsafe {
        compute_q(p_stark, p_params, pProof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_evals_c(p_stark: *mut c_void, p_params: *mut c_void, pProof: *mut c_void) {
    unsafe {
        compute_evals(p_stark, p_params, pProof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_pol_c(
    p_stark: *mut c_void,
    p_params: *mut c_void,
    steps: *mut c_void,
    nrowsStepBatch: u64,
) -> *mut c_void {
    unsafe { compute_fri_pol(p_stark, p_params, steps, nrowsStepBatch) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_folding_c(
    p_stark: *mut c_void,
    pProof: *mut c_void,
    pFriPol: *mut c_void,
    step: u64,
    challenge: *mut c_void,
) {
    unsafe {
        compute_fri_folding(p_stark, pProof, pFriPol, step, challenge);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_queries_c(p_stark: *mut c_void, pProof: *mut c_void, pFriPol: *mut c_void, friQueries: *mut u64) {
    unsafe {
        compute_fri_queries(p_stark, pProof, pFriPol, friQueries);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_num_rows_step_batch_c(p_stark: *mut c_void) -> u64 {
    unsafe { get_num_rows_step_batch(p_stark) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_pols_starks_new_c(
    p_address: *mut c_void,
    degree: u64,
    n_committed_pols: u64,
) -> *mut std::os::raw::c_void {
    unsafe { commit_pols_starks_new(p_address, degree, n_committed_pols) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_pols_starks_free_c(p_commit_pols_starks: *mut c_void) {
    unsafe {
        commit_pols_starks_free(p_commit_pols_starks);
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
pub fn zkin_new_c<T>(
    p_starks: *mut c_void,
    p_fri_proof: *mut c_void,
    public_inputs: &Vec<T>,
    root_c: &Vec<Goldilocks>,
) -> *mut c_void {
    unsafe {
        zkin_new(
            p_starks,
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
pub fn transcript_add_c(p_transcript: *mut c_void, p_input: *mut c_void, size: u64) {
    unsafe {
        transcript_add(p_transcript, p_input, size);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_add_polinomial_c(p_transcript: *mut c_void, p_polinomial: *mut c_void) {
    unsafe {
        transcript_add_polinomial(p_transcript, p_polinomial);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_get_field_c(p_transcript: *mut c_void, pOutput: *mut c_void) {
    unsafe {
        transcript_get_field(p_transcript, pOutput);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_free_c(p_transcript: *mut c_void) {
    unsafe {
        transcript_free(p_transcript);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_challenges_c(p_transcript: *mut c_void, p_polinomial: *mut c_void, n_challenges: u64, index: u64) {
    unsafe {
        get_challenges(p_transcript, p_polinomial, n_challenges, index);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_permutations_c(p_transcript: *mut c_void, res: *mut u64, n: u64, n_bits: u64) {
    unsafe {
        get_permutations(p_transcript, res, n, n_bits);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_new_c(degree: u64, dim: u64, name: &str) -> *mut c_void {
    unsafe {
        let name = CString::new(name).unwrap();

        polinomial_new(degree, dim, name.as_ptr() as *mut std::os::raw::c_char)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_new_void_c() -> *mut c_void {
    unsafe { polinomial_new_void() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_get_address_c(p_polinomial: *mut c_void) -> *mut c_void {
    unsafe { polinomial_get_address(p_polinomial) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_get_p_element_c(p_polinomial: *mut c_void, index: u64) -> *mut c_void {
    unsafe { polinomial_get_p_element(p_polinomial, index) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn polinomial_free_c(p_polinomial: *mut c_void) {
    unsafe {
        polinomial_free(p_polinomial);
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
pub fn zkevm_steps_free_c(_p_zkevm_steps: *mut c_void) {
    println!("zkevm_steps_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn c12a_steps_new_c() -> *mut std::os::raw::c_void {
    println!("c12a_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn c12a_steps_free_c(_p_c12a_steps: *mut c_void) {
    println!("c12a_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn recursive1_steps_new_c() -> *mut std::os::raw::c_void {
    println!("recursive1_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn recursive1_steps_free_c(_p_recursive1_steps: *mut c_void) {
    println!("recursive1_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn _recursive2_steps_new_c() -> *mut std::os::raw::c_void {
    println!("recursive2_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn _recursive2_steps_free_c(_p_recursive2_steps: *mut c_void) {
    println!("recursive2_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step2prev_parser_first_avx_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step2prev_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step2prev_parser_first_avx512_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step2prev_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step2prev_first_parallel_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64) {
    println!("step2prev_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3prev_parser_first_avx_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step3prev_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3prev_parser_first_avx512_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step3prev_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3prev_first_parallel_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64) {
    println!("step3prev_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3_parser_first_avx_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step3_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3_parser_first_avx512_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step3_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step3_first_parallel_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64) {
    println!("step3_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step42ns_parser_first_avx_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step42ns_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step42ns_parser_first_avx512_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step42ns_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step42ns_first_parallel_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64) {
    println!("step42ns_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step52ns_parser_first_avx_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step52ns_parser_first_avx: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step52ns_parser_first_avx512_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64, _n_rows_batch: u64) {
    println!("step52ns_parser_first_avx512: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn step52ns_first_parallel_c(_p_steps: *mut c_void, _p_params: *mut c_void, _nrows: u64) {
    println!("step52ns_first_parallel: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_new_c(_p_starks: *mut c_void) -> *mut c_void {
    println!("fri_proof_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_get_root_c(_pFriProof: *mut c_void, _root_index: u64, _root_subindex: u64) -> *mut c_void {
    println!("fri_proof_get_root: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_get_tree_root_c(_pFriProof: *mut c_void, _tree_index: u64, _root_index: u64) -> *mut c_void {
    println!("fri_proof_get_tree_root: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_free_c(_p_zkevm_steps: *mut c_void) {
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
pub fn get_stark_info_c(_p_stark: *mut c_void) -> *mut c_void {
    println!("get_stark_info: This is a mock call because there is no linked library");
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
pub fn starks_free_c(_p_stark: *mut c_void) {
    println!("starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn steps_params_new_c(
    _p_stark: *mut c_void,
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
pub fn steps_params_free_c(_p_stepsParams: *mut c_void) {
    println!("steps_params_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn extend_and_merkelize_c(_p_stark: *mut c_void, _step: u64, _p_params: *mut c_void, _proof: *mut c_void) {
    println!("extend_and_merkelize: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_h1_h2_c(_p_stark: *mut c_void, _p_params: *mut c_void) {
    println!("calculate_h1_h2: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_z_c(_p_stark: *mut c_void, _p_params: *mut c_void) {
    println!("calculate_z: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_expressions_c(
    _p_stark: *mut c_void,
    _step: &str,
    _nrowsStepBatch: u64,
    _p_steps: *mut c_void,
    _p_params: *mut c_void,
    _n: u64,
) {
    println!("calculate_expressions: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_q_c(_p_stark: *mut c_void, _p_params: *mut c_void, _pProof: *mut c_void) {
    println!("compute_q: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_evals_c(_p_stark: *mut c_void, _p_params: *mut c_void, _pProof: *mut c_void) {
    println!("compute_evals: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_pol_c(
    _p_stark: *mut c_void,
    _p_params: *mut c_void,
    _steps: *mut c_void,
    _nrowsStepBatch: u64,
) -> *mut c_void {
    println!("compute_fri_pol: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_folding_c(
    _p_stark: *mut c_void,
    _pProof: *mut c_void,
    _pFriPol: *mut c_void,
    _step: u64,
    _challenge: *mut c_void,
) {
    println!("compute_fri_folding: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_queries_c(
    _p_stark: *mut c_void,
    _pProof: *mut c_void,
    _pFriPol: *mut c_void,
    _friQueries: *mut u64,
) {
    println!("compute_fri_queries: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_num_rows_step_batch_c(_p_stark: *mut c_void) -> u64 {
    println!("get_num_rows_step_batch: This is a mock call because there is no linked library");
    1
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_starks_new_c(
    _p_address: *mut c_void,
    _degree: u64,
    _n_committed_pols: u64,
) -> *mut std::os::raw::c_void {
    println!("commit_pols_starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_starks_free_c(_p_commit_pols_starks: *mut c_void) {
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
pub fn zkin_new_c<T>(
    _p_starks: *mut c_void,
    _p_fri_proof: *mut c_void,
    _public_inputs: &Vec<T>,
    _root_c: &Vec<Goldilocks>,
) -> *mut c_void {
    println!("zkin_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_new_c() -> *mut c_void {
    println!("transcript_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_add_c(_p_transcript: *mut c_void, _p_input: *mut c_void, _size: u64) {
    println!("transcript_add: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_add_polinomial_c(_p_transcript: *mut c_void, _p_polinomial: *mut c_void) {
    println!("transcript_add_polinomial: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_get_field_c(_p_transcript: *mut c_void, _pOutput: *mut c_void) {
    println!("transcript_get_field: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_free_c(_p_transcript: *mut c_void) {
    println!("transcript_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_challenges_c(_p_transcript: *mut c_void, _p_polinomial: *mut c_void, _n_challenges: u64, _index: u64) {
    println!("get_challenges: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_permutations_c(_p_transcript: *mut c_void, _res: *mut u64, _n: u64, _n_bits: u64) {
    println!("get_permutations: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_new_c(_degree: u64, _dim: u64, _name: &str) -> *mut c_void {
    println!("polinomial_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_new_void_c() -> *mut c_void {
    println!("polinomial_new_void: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_get_address_c(_p_polinomial: *mut c_void) -> *mut c_void {
    println!("get_address: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_get_p_element_c(_p_polinomial: *mut c_void, _index: u64) -> *mut c_void {
    println!("get_p_element: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn polinomial_free_c(_p_polinomial: *mut c_void) {
    println!("polinomial_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_new_c(_p_address: *mut c_void, _degree: u64) -> *mut c_void {
    println!("commit_pols_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_free_c(_pCommitPols: *mut c_void) {
    println!("commit_pols_free: This is a mock call because there is no linked library");
}
