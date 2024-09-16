#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use ::std::os::raw::c_void;

#[cfg(feature = "no_lib_link")]
use log::trace;

#[cfg(not(feature = "no_lib_link"))]
include!("../bindings_starks.rs");

#[cfg(not(feature = "no_lib_link"))]
use std::ffi::CString;

#[cfg(not(feature = "no_lib_link"))]
pub fn save_challenges_c(p_challenges: *mut std::os::raw::c_void, global_info_file: &str, output_dir: &str) {
    unsafe {
        let file_dir = CString::new(output_dir).unwrap();
        let file_ptr = file_dir.as_ptr() as *mut std::os::raw::c_char;

        let global_info_file_name = CString::new(global_info_file).unwrap();
        let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

        save_challenges(p_challenges as *mut std::os::raw::c_void, global_info_file_ptr, file_ptr);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn save_publics_c(n_publics: u64, public_inputs: *mut std::os::raw::c_void, output_dir: &str) {
    let file_dir: CString = CString::new(output_dir).unwrap();
    unsafe {
        save_publics(n_publics, public_inputs, file_dir.as_ptr() as *mut std::os::raw::c_char);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn save_proof_c(proof_id: u64, p_stark_info: *mut c_void, p_fri_proof: *mut c_void, output_dir: &str) {
    let file_dir = CString::new(output_dir).unwrap();
    unsafe {
        save_proof(proof_id, p_stark_info, p_fri_proof, file_dir.as_ptr() as *mut std::os::raw::c_char);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_new_c(p_setup_ctx: *mut c_void) -> *mut c_void {
    unsafe { fri_proof_new(p_setup_ctx) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_get_tree_root_c(p_fri_proof: *mut c_void, tree_index: u64, root_index: u64) -> *mut c_void {
    unsafe { fri_proof_get_tree_root(p_fri_proof, tree_index, root_index) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_set_subproof_values_c(p_fri_proof: *mut c_void, p_subproof_values: *mut c_void) {
    unsafe { fri_proof_set_subproofvalues(p_fri_proof, p_subproof_values) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_free_c(p_fri_proof: *mut c_void) {
    unsafe {
        fri_proof_free(p_fri_proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn setup_ctx_new_c(
    p_stark_info: *mut c_void,
    p_expressions_bin: *mut c_void,
    p_const_pols: *mut c_void,
) -> *mut c_void {
    unsafe { setup_ctx_new(p_stark_info, p_expressions_bin, p_const_pols) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn setup_ctx_free_c(p_setup_ctx: *mut c_void) {
    unsafe {
        setup_ctx_free(p_setup_ctx);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn stark_info_new_c(filename: &str) -> *mut c_void {
    unsafe {
        let filename = CString::new(filename).unwrap();

        stark_info_new(filename.as_ptr() as *mut std::os::raw::c_char)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_map_totaln_c(p_stark_info: *mut ::std::os::raw::c_void) -> u64 {
    unsafe { get_map_total_n(p_stark_info) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_map_offsets_c(pStarkInfo: *mut c_void, stage: &str, flag: bool) -> u64 {
    let stage = CString::new(stage).unwrap();
    unsafe { get_map_offsets(pStarkInfo, stage.as_ptr() as *mut std::os::raw::c_char, flag) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn stark_info_free_c(p_stark_info: *mut c_void) {
    unsafe {
        stark_info_free(p_stark_info);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn const_pols_new_c(filename: &str, p_stark_info: *mut c_void) -> *mut c_void {
    unsafe {
        let filename = CString::new(filename).unwrap();

        const_pols_new(filename.as_ptr() as *mut std::os::raw::c_char, p_stark_info)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn const_pols_free_c(p_const_pols: *mut c_void) {
    unsafe {
        const_pols_free(p_const_pols);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn expressions_bin_new_c(filename: &str) -> *mut c_void {
    unsafe {
        let filename = CString::new(filename).unwrap();

        expressions_bin_new(filename.as_ptr() as *mut std::os::raw::c_char)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn expressions_bin_free_c(p_expressions_bin: *mut c_void) {
    unsafe {
        expressions_bin_free(p_expressions_bin);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_ids_by_name_c(p_setup: *mut c_void, hint_name: &str) -> *mut c_void {
    let name = CString::new(hint_name).unwrap();
    unsafe { get_hint_ids_by_name(p_setup, name.as_ptr() as *mut std::os::raw::c_char) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_c(
    p_setup_ctx: *mut c_void,
    buffer: *mut c_void,
    public_inputs: *mut c_void,
    challenges: *mut c_void,
    subproofValues: *mut c_void,
    evals: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
    dest: bool,
    inverse: bool,
    print_expression: bool,
) -> *mut c_void {
    let field_name = CString::new(hint_field_name).unwrap();
    unsafe {
        get_hint_field(
            p_setup_ctx,
            buffer,
            public_inputs,
            challenges,
            subproofValues,
            evals,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
            dest,
            inverse,
            print_expression,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn set_hint_field_c(
    p_setup_ctx: *mut c_void,
    buffer: *mut c_void,
    subproof_values: *mut c_void,
    values: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
) -> u64 {
    unsafe {
        let field_name = CString::new(hint_field_name).unwrap();
        set_hint_field(
            p_setup_ctx,
            buffer,
            subproof_values,
            values,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_new_c(p_setup_ctx: *mut c_void) -> *mut c_void {
    unsafe { starks_new(p_setup_ctx) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_free_c(p_stark: *mut c_void) {
    unsafe {
        starks_free(p_stark);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn extend_and_merkelize_c(p_stark: *mut c_void, step: u64, buffer: *mut c_void, proof: *mut c_void) {
    unsafe {
        extend_and_merkelize(p_stark, step, buffer, proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn treesGL_get_root_c(pStark: *mut c_void, index: u64, root: *mut c_void) {
    unsafe {
        treesGL_get_root(pStark, index, root);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn prepare_fri_polynomial_c(p_stark: *mut c_void, buffer: *mut c_void, challenges: *mut c_void) -> *mut c_void {
    unsafe { prepare_fri_pol(p_stark, buffer, challenges) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_fri_pol_c(p_setup_ctx: *mut c_void, buffer: *mut c_void) -> *mut c_void {
    unsafe { get_fri_pol(p_setup_ctx, buffer) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_fri_polynomial_c(
    p_starks: *mut c_void,
    buffer: *mut c_void,
    public_inputs: *mut c_void,
    challenges: *mut c_void,
    subproofValues: *mut c_void,
    evals: *mut c_void,
) {
    unsafe {
        calculate_fri_polynomial(p_starks, buffer, public_inputs, challenges, subproofValues, evals);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_quotient_polynomial_c(
    p_starks: *mut c_void,
    buffer: *mut c_void,
    public_inputs: *mut c_void,
    challenges: *mut c_void,
    subproofValues: *mut c_void,
    evals: *mut c_void,
) {
    unsafe {
        calculate_quotient_polynomial(p_starks, buffer, public_inputs, challenges, subproofValues, evals);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_impols_expressions_c(
    p_starks: *mut c_void,
    step: u64,
    buffer: *mut c_void,
    public_inputs: *mut c_void,
    challenges: *mut c_void,
    subproofValues: *mut c_void,
    evals: *mut c_void,
) {
    unsafe {
        calculate_impols_expressions(p_starks, step, buffer, public_inputs, challenges, subproofValues, evals);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_stage_c(p_starks: *mut c_void, element_type: u32, step: u64, buffer: *mut c_void, p_proof: *mut c_void) {
    unsafe {
        commit_stage(p_starks, element_type, step, buffer, p_proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_evals_c(
    p_stark: *mut c_void,
    buffer: *mut c_void,
    challenges: *mut c_void,
    evals: *mut c_void,
    pProof: *mut c_void,
) {
    unsafe {
        compute_evals(p_stark, buffer, challenges, evals, pProof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_folding_c(
    p_stark: *mut c_void,
    step: u64,
    pProof: *mut c_void,
    buffer: *mut c_void,
    challenge: *mut c_void,
) {
    unsafe {
        compute_fri_folding(p_stark, pProof, step, buffer, challenge);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_queries_c(p_stark: *mut c_void, p_proof: *mut c_void, p_fri_queries: *mut u64) {
    unsafe {
        compute_fri_queries(p_stark, p_proof, p_fri_queries);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_hash_c(pStarks: *mut c_void, pHhash: *mut c_void, pBuffer: *mut c_void, nElements: u64) {
    unsafe {
        calculate_hash(pStarks, pHhash, pBuffer, nElements);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_new_c(element_type: u32, arity: u64, custom: bool) -> *mut c_void {
    unsafe { transcript_new(element_type, arity, custom) }
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
pub fn transcript_free_c(p_transcript: *mut c_void, type_: u32) {
    unsafe {
        transcript_free(p_transcript, type_);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_challenge_c(p_starks: *mut c_void, p_transcript: *mut c_void, p_element: *mut c_void) {
    unsafe {
        get_challenge(p_starks, p_transcript, p_element);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_permutations_c(p_transcript: *mut c_void, res: *mut u64, n: u64, n_bits: u64) {
    unsafe {
        get_permutations(p_transcript, res, n, n_bits);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn verify_constraints_c(
    p_setup: *mut c_void,
    buffer: *mut c_void,
    public_inputs: *mut c_void,
    challenges: *mut c_void,
    subproofValues: *mut c_void,
    evals: *mut c_void,
) -> *mut c_void {
    unsafe { verify_constraints(p_setup, buffer, public_inputs, challenges, subproofValues, evals) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn verify_global_constraints_c(
    global_info_file: &str,
    global_constraints_bin_file: &str,
    publics: *mut c_void,
    proofs: *mut c_void,
    n_proofs: u64,
) -> bool {
    unsafe {
        let global_info_file_name = CString::new(global_info_file).unwrap();
        let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

        let global_constraints_bin_file_name = CString::new(global_constraints_bin_file).unwrap();
        let global_constraints_bin_file_ptr = global_constraints_bin_file_name.as_ptr() as *mut std::os::raw::c_char;

        verify_global_constraints(global_info_file_ptr, global_constraints_bin_file_ptr, publics, proofs, n_proofs)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn print_by_name_c(
    p_setup_ctx: *mut c_void,
    buffer: *mut c_void,
    public_inputs: *mut c_void,
    challenges: *mut c_void,
    subproof_values: *mut c_void,
    name: &str,
    lengths: *mut u64,
    first_print_value: u64,
    last_print_value: u64,
    return_values: bool,
) -> *mut c_void {
    let name_string = CString::new(name).unwrap();
    let name_ptr = name_string.as_ptr() as *mut std::os::raw::c_char;
    unsafe {
        print_by_name(
            p_setup_ctx,
            buffer,
            public_inputs,
            challenges,
            subproof_values,
            name_ptr,
            lengths,
            first_print_value,
            last_print_value,
            return_values,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn print_expression_c(
    p_setup_ctx: *mut c_void,
    pol: *mut c_void,
    dim: u64,
    first_print_value: u64,
    last_print_value: u64,
) {
    unsafe {
        print_expression(p_setup_ctx, pol, dim, first_print_value, last_print_value);
    }
}

// ------------------------
// MOCK METHODS FOR TESTING
// ------------------------
#[cfg(feature = "no_lib_link")]
pub fn save_challenges_c(_p_challenges: *mut std::os::raw::c_void, _global_info_file: &str, _output_dir: &str) {
    trace!("{}: ··· {}", "ffi     ", "save_challenges_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn save_publics_c(_n_publics: u64, _public_inputs: *mut c_void, _output_dir: &str) {
    trace!("{}: ··· {}", "ffi     ", "save_publics_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn save_proof_c(_proof_id: u64, _p_stark_info: *mut c_void, _p_fri_proof: *mut c_void, _output_dir: &str) {
    trace!("{}: ··· {}", "ffi     ", "save_proof: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_new_c(_p_setup_ctx: *mut c_void) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_get_tree_root_c(_p_fri_proof: *mut c_void, _tree_index: u64, _root_index: u64) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_get_tree_root: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_set_subproof_values_c(_p_fri_proof: *mut c_void, _p_params: *mut c_void) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "fri_proof_set_subproof_values_c: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_free_c(_p_fri_proof: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn setup_ctx_new_c(
    _p_stark_info: *mut c_void,
    _p_expressions_bin: *mut c_void,
    _p_const_pols: *mut c_void,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_free: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn setup_ctx_new1_c(
    _stark_info_file: &str,
    _expressions_bin_file: &str,
    _const_pols_file: &str,
    _const_tree_file: &str,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_free: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn setup_ctx_free_c(_p_setup_ctx: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "starkinfo_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn stark_info_new_c(_filename: &str) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "starkinfo_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_map_totaln_c(_p_stark_info: *mut ::std::os::raw::c_void) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_map_totaln_c: This is a mock call because there is no linked library");
    10000
}

#[cfg(feature = "no_lib_link")]
pub fn get_map_offsets_c(_p_stark_info: *mut c_void, _stage: &str, _flag: bool) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_map_offsets: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn stark_info_free_c(_p_stark_info: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "starkinfo_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn const_pols_new_c(_filename: &str, _p_stark_info: *mut c_void) -> *mut c_void {
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn const_pols_free_c(_p_const_pols: *mut c_void) {}

#[cfg(feature = "no_lib_link")]
pub fn expressions_bin_new_c(_filename: &str) -> *mut c_void {
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn expressions_bin_free_c(_p_expressions_bin: *mut c_void) {}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_ids_by_name_c(_p_setup_ctx: *mut c_void, _hint_name: &str) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "get_hint_ids_by_name: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_c(
    _p_setup_ctx: *mut c_void,
    _buffer: *mut c_void,
    _public_inputs: *mut c_void,
    _challenges: *mut c_void,
    _subproofValues: *mut c_void,
    _evals: *mut c_void,
    _hint_id: u64,
    _hint_field_name: &str,
    _dest: bool,
    _inverse: bool,
    _print_expression: bool,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "get_hint_field: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn set_hint_field_c(
    _p_setup_ctx: *mut c_void,
    _buffer: *mut c_void,
    _subproof_values: *mut c_void,
    _values: *mut c_void,
    _hint_id: u64,
    _hint_field_name: &str,
) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "set_hint_field_c: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn starks_new_c(_p_config: *mut c_void) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn starks_free_c(_p_stark: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn extend_and_merkelize_c(_p_stark: *mut c_void, _step: u64, _buffer: *mut c_void, _proof: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "extend_and_merkelize: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn treesGL_get_root_c(_pStark: *mut c_void, _index: u64, _root: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "treesGL_get_root: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_fri_polynomial_c(
    _p_starks: *mut c_void,
    _buffer: *mut c_void,
    _public_inputs: *mut c_void,
    _challenges: *mut c_void,
    _subproofValues: *mut c_void,
    _evals: *mut c_void,
) {
    trace!("mckzkevm: ··· {}", "calculate_fri_polynomial: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_quotient_polynomial_c(
    _p_starks: *mut c_void,
    _buffer: *mut c_void,
    _public_inputs: *mut c_void,
    _challenges: *mut c_void,
    _subproofValues: *mut c_void,
    _evals: *mut c_void,
) {
    trace!("mckzkevm: ··· {}", "calculate_quotient_polynomial: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_impols_expressions_c(
    _p_starks: *mut c_void,
    _step: u64,
    _buffer: *mut c_void,
    _public_inputs: *mut c_void,
    _challenges: *mut c_void,
    _subproofValues: *mut c_void,
    _evals: *mut c_void,
) {
    trace!(
        "{}: ··· {}",
        "mckzkevm",
        "calculate_impols_expression: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn commit_stage_c(
    _p_starks: *mut c_void,
    _element_type: u32,
    _step: u64,
    _buffer: *mut c_void,
    _p_proof: *mut c_void,
) {
    trace!("{}: ··· {}", "ffi     ", "commit_stage: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_evals_c(
    _p_stark: *mut c_void,
    _buffer: *mut c_void,
    _challenges: *mut c_void,
    _evals: *mut c_void,
    _pProof: *mut c_void,
) {
    trace!("{}: ··· {}", "ffi     ", "compute_evals: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn prepare_fri_polynomial_c(_p_stark: *mut c_void, _buffer: *mut c_void, _challenges: *mut c_void) -> *mut c_void {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "prepare_fri_polynomial_c: This is a mock call because there is no linked library"
    );
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_fri_pol_c(_p_setup_ctx: *mut c_void, _buffer: *mut c_void) -> *mut c_void {
    trace!("ffi     : ··· {}", "get_fri_pol: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_folding_c(
    _p_stark: *mut c_void,
    _step: u64,
    _p_proof: *mut c_void,
    _buffer: *mut c_void,
    _challenge: *mut c_void,
) {
    trace!("{}: ··· {}", "ffi     ", "compute_fri_folding: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_queries_c(_p_stark: *mut c_void, _p_proof: *mut c_void, _p_fri_queries: *mut u64) {
    trace!("{}: ··· {}", "ffi     ", "compute_fri_queries: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_hash_c(_pStarks: *mut c_void, _pHhash: *mut c_void, _pBuffer: *mut c_void, _nElements: u64) {
    trace!("{}: ··· {}", "ffi     ", "calculate_hash: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_new_c(_element_type: u32, _arity: u64, _custom: bool) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "transcript_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_add_c(_p_transcript: *mut c_void, _p_input: *mut c_void, _size: u64) {
    trace!("{}: ··· {}", "ffi     ", "transcript_add: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_add_polinomial_c(_p_transcript: *mut c_void, _p_polinomial: *mut c_void) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "transcript_add_polinomial: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_free_c(_p_transcript: *mut c_void, _element_type: u32) {
    trace!("{}: ··· {}", "ffi     ", "transcript_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_challenge_c(_p_starks: *mut c_void, _p_transcript: *mut c_void, _p_element: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "get_challenges: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_permutations_c(_p_transcript: *mut c_void, _res: *mut u64, _n: u64, _n_bits: u64) {
    trace!("{}: ··· {}", "ffi     ", "get_permutations: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn verify_constraints_c(
    _p_setup: *mut c_void,
    _buffer: *mut c_void,
    _public_inputs: *mut c_void,
    _challenges: *mut c_void,
    _subproofValues: *mut c_void,
    _evals: *mut c_void,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "verify_constraints_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn verify_global_constraints_c(
    _global_info_file: &str,
    _global_constraints_bin_file: &str,
    _publics: *mut c_void,
    _proofs: *mut c_void,
    _n_proofs: u64,
) -> bool {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "verify_global_constraints_c: This is a mock call because there is no linked library"
    );
    true
}

#[cfg(feature = "no_lib_link")]
pub fn print_by_name_c(
    _p_setup_ctx: *mut c_void,
    _buffer: *mut c_void,
    _public_inputs: *mut c_void,
    _challenges: *mut c_void,
    _subproof_values: *mut c_void,
    _name: &str,
    _lengths: *mut u64,
    _first_print_value: u64,
    _last_print_value: u64,
    _return_values: bool,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "print_by_name_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn print_expression_c(
    _p_setup_ctx: *mut c_void,
    _pol: *mut c_void,
    _dim: u64,
    _first_print_value: u64,
    _last_print_value: u64,
) {
    trace!("{}: ··· {}", "ffi     ", "print_expression_c: This is a mock call because there is no linked library");
}
