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
pub fn save_challenges_c(p_challenges: *mut u8, global_info_file: &str, output_dir: &str) {
    unsafe {
        let file_dir = CString::new(output_dir).unwrap();
        let file_ptr = file_dir.as_ptr() as *mut std::os::raw::c_char;

        let global_info_file_name = CString::new(global_info_file).unwrap();
        let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

        save_challenges(p_challenges as *mut std::os::raw::c_void, global_info_file_ptr, file_ptr);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn save_publics_c(n_publics: u64, public_inputs: *mut u8, output_dir: &str) {
    let file_dir: CString = CString::new(output_dir).unwrap();
    unsafe {
        save_publics(
            n_publics,
            public_inputs as *mut std::os::raw::c_void,
            file_dir.as_ptr() as *mut std::os::raw::c_char,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn save_proof_values_c(proof_values: *mut u8, global_info_file: &str, output_dir: &str) {
    let file_dir: CString = CString::new(output_dir).unwrap();

    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        save_proof_values(
            proof_values as *mut std::os::raw::c_void,
            global_info_file_ptr,
            file_dir.as_ptr() as *mut std::os::raw::c_char,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_new_c(p_setup_ctx: *mut c_void, instance_id: u64) -> *mut c_void {
    unsafe { fri_proof_new(p_setup_ctx, instance_id) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_get_tree_root_c(p_fri_proof: *mut c_void, root: *mut u8, tree_index: u64) {
    unsafe {
        fri_proof_get_tree_root(p_fri_proof, root as *mut std::os::raw::c_void, tree_index);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_set_airgroup_values_c(p_fri_proof: *mut c_void, p_airgroup_values: *mut u8) {
    unsafe { fri_proof_set_airgroupvalues(p_fri_proof, p_airgroup_values as *mut std::os::raw::c_void) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_set_air_values_c(p_fri_proof: *mut c_void, p_air_values: *mut u8) {
    unsafe { fri_proof_set_airvalues(p_fri_proof, p_air_values as *mut std::os::raw::c_void) }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn fri_proof_get_zkinproofs_c(
    n_proofs: u64,
    proofs: *mut *mut c_void,
    fri_proofs: *mut *mut c_void,
    p_publics: *mut u8,
    p_proof_values: *mut u8,
    p_challenges: *mut u8,
    global_info_file: &str,
    output_dir_file: &str,
) {
    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    let file_dir = CString::new(output_dir_file).unwrap();
    let file_ptr = file_dir.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        fri_proof_get_zkinproofs(
            n_proofs,
            proofs,
            fri_proofs,
            p_publics as *mut std::os::raw::c_void,
            p_proof_values as *mut std::os::raw::c_void,
            p_challenges as *mut std::os::raw::c_void,
            global_info_file_ptr,
            file_ptr,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn fri_proof_get_zkinproof_c(
    p_fri_proof: *mut c_void,
    p_publics: *mut u8,
    p_challenges: *mut u8,
    p_proof_values: *mut u8,
    global_info_file: &str,
    output_dir_file: &str,
) -> *mut c_void {
    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    let file_dir = CString::new(output_dir_file).unwrap();
    let file_ptr = file_dir.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        fri_proof_get_zkinproof(
            p_fri_proof,
            p_publics as *mut std::os::raw::c_void,
            p_challenges as *mut std::os::raw::c_void,
            p_proof_values as *mut std::os::raw::c_void,
            global_info_file_ptr,
            file_ptr,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_free_zkinproof_c(p_fri_proof: *mut c_void) {
    unsafe {
        fri_proof_free_zkinproof(p_fri_proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn fri_proof_free_c(p_fri_proof: *mut c_void) {
    unsafe {
        fri_proof_free(p_fri_proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn stark_info_new_c(filename: &str, verify: bool) -> *mut c_void {
    unsafe {
        let filename = CString::new(filename).unwrap();

        stark_info_new(filename.as_ptr() as *mut std::os::raw::c_char, verify)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_map_totaln_c(p_stark_info: *mut c_void, recursive: bool) -> u64 {
    unsafe { get_map_total_n(p_stark_info, recursive) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn stark_info_free_c(p_stark_info: *mut c_void) {
    unsafe {
        stark_info_free(p_stark_info);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn prover_helpers_new_c(p_stark_info: *mut c_void, pil1: bool) -> *mut c_void {
    unsafe { prover_helpers_new(p_stark_info, pil1) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn prover_helpers_free_c(p_prover_helpers: *mut c_void) {
    unsafe {
        prover_helpers_free(p_prover_helpers);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn load_const_pols_c(pConstPolsAddress: *mut u8, const_filename: &str, const_size: u64) {
    unsafe {
        let const_filename: CString = CString::new(const_filename).unwrap();

        load_const_pols(
            pConstPolsAddress as *mut std::os::raw::c_void,
            const_filename.as_ptr() as *mut std::os::raw::c_char,
            const_size,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_const_size_c(pStarkInfo: *mut c_void) -> u64 {
    unsafe { get_const_size(pStarkInfo) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_const_tree_size_c(pStarkInfo: *mut c_void) -> u64 {
    unsafe { get_const_tree_size(pStarkInfo) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn load_const_tree_c(pConstPolsTreeAddress: *mut u8, tree_filename: &str, const_tree_size: u64) {
    unsafe {
        let tree_filename: CString = CString::new(tree_filename).unwrap();

        load_const_tree(
            pConstPolsTreeAddress as *mut std::os::raw::c_void,
            tree_filename.as_ptr() as *mut std::os::raw::c_char,
            const_tree_size,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_const_tree_c(
    pStarkInfo: *mut c_void,
    pConstPols: *mut u8,
    pConstPolsTreeAddress: *mut u8,
    tree_filename: &str,
) {
    unsafe {
        let tree_filename: CString = CString::new(tree_filename).unwrap();

        calculate_const_tree(
            pStarkInfo,
            pConstPols as *mut std::os::raw::c_void,
            pConstPolsTreeAddress as *mut std::os::raw::c_void,
            tree_filename.as_ptr() as *mut std::os::raw::c_char,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn expressions_bin_new_c(filename: &str, global: bool, verify: bool) -> *mut c_void {
    unsafe {
        let filename = CString::new(filename).unwrap();

        expressions_bin_new(filename.as_ptr() as *mut std::os::raw::c_char, global, verify)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn expressions_bin_free_c(p_expressions_bin: *mut c_void) {
    unsafe {
        expressions_bin_free(p_expressions_bin);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn n_hint_ids_by_name_c(p_expressions_bin: *mut c_void, hint_name: &str) -> u64 {
    let name = CString::new(hint_name).unwrap();
    unsafe { n_hints_by_name(p_expressions_bin, name.as_ptr() as *mut std::os::raw::c_char) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_ids_by_name_c(p_expressions_bin: *mut c_void, hint_ids: *mut u64, hint_name: &str) {
    let name = CString::new(hint_name).unwrap();
    unsafe {
        get_hint_ids_by_name(p_expressions_bin, hint_ids, name.as_ptr() as *mut std::os::raw::c_char);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_c(
    p_setup_ctx: *mut c_void,
    p_steps_params: *mut u8,
    hint_field_values: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
    hint_options: *mut u8,
) {
    let field_name = CString::new(hint_field_name).unwrap();
    unsafe {
        get_hint_field(
            p_setup_ctx,
            p_steps_params as *mut std::os::raw::c_void,
            hint_field_values,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
            hint_options as *mut std::os::raw::c_void,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_values_c(p_setup_ctx: *mut c_void, hint_id: u64, hint_field_name: &str) -> u64 {
    let field_name = CString::new(hint_field_name).unwrap();
    unsafe { get_hint_field_values(p_setup_ctx, hint_id, field_name.as_ptr() as *mut std::os::raw::c_char) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_sizes_c(
    p_setup_ctx: *mut c_void,
    hint_field_values: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
    hint_options: *mut u8,
) {
    let field_name = CString::new(hint_field_name).unwrap();
    unsafe {
        get_hint_field_sizes(
            p_setup_ctx,
            hint_field_values,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
            hint_options as *mut std::os::raw::c_void,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn mul_hint_fields_c(
    p_setup_ctx: *mut c_void,
    p_steps_params: *mut u8,
    hint_id: u64,
    hint_field_dest: &str,
    hint_field_name1: &str,
    hint_field_name2: &str,
    hint_options1: *mut u8,
    hint_options2: *mut u8,
) -> u64 {
    let field_dest = CString::new(hint_field_dest).unwrap();
    let field_name1 = CString::new(hint_field_name1).unwrap();
    let field_name2 = CString::new(hint_field_name2).unwrap();

    unsafe {
        mul_hint_fields(
            p_setup_ctx,
            p_steps_params as *mut std::os::raw::c_void,
            hint_id,
            field_dest.as_ptr() as *mut std::os::raw::c_char,
            field_name1.as_ptr() as *mut std::os::raw::c_char,
            field_name2.as_ptr() as *mut std::os::raw::c_char,
            hint_options1 as *mut std::os::raw::c_void,
            hint_options2 as *mut std::os::raw::c_void,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn acc_hint_field_c(
    p_setup_ctx: *mut c_void,
    p_steps_params: *mut u8,
    hint_id: u64,
    hint_field_dest: &str,
    hint_field_airgroupvalue: &str,
    hint_field_name: &str,
    add: bool,
) {
    let field_dest = CString::new(hint_field_dest).unwrap();
    let field_airgroupvalue = CString::new(hint_field_airgroupvalue).unwrap();
    let field_name = CString::new(hint_field_name).unwrap();

    unsafe {
        acc_hint_field(
            p_setup_ctx,
            p_steps_params as *mut std::os::raw::c_void,
            hint_id,
            field_dest.as_ptr() as *mut std::os::raw::c_char,
            field_airgroupvalue.as_ptr() as *mut std::os::raw::c_char,
            field_name.as_ptr() as *mut std::os::raw::c_char,
            add,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn acc_mul_hint_fields_c(
    p_setup_ctx: *mut c_void,
    p_steps_params: *mut u8,
    hint_id: u64,
    hint_field_dest: &str,
    hint_field_airgroupvalue: &str,
    hint_field_name1: &str,
    hint_field_name2: &str,
    hint_options1: *mut u8,
    hint_options2: *mut u8,
    add: bool,
) {
    let field_dest = CString::new(hint_field_dest).unwrap();
    let field_airgroupvalue = CString::new(hint_field_airgroupvalue).unwrap();
    let field_name1 = CString::new(hint_field_name1).unwrap();
    let field_name2: CString = CString::new(hint_field_name2).unwrap();

    unsafe {
        acc_mul_hint_fields(
            p_setup_ctx,
            p_steps_params as *mut std::os::raw::c_void,
            hint_id,
            field_dest.as_ptr() as *mut std::os::raw::c_char,
            field_airgroupvalue.as_ptr() as *mut std::os::raw::c_char,
            field_name1.as_ptr() as *mut std::os::raw::c_char,
            field_name2.as_ptr() as *mut std::os::raw::c_char,
            hint_options1 as *mut std::os::raw::c_void,
            hint_options2 as *mut std::os::raw::c_void,
            add,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn update_airgroupvalue_c(
    p_setup_ctx: *mut c_void,
    p_steps_params: *mut u8,
    hint_id: u64,
    hint_field_airgroupvalue: &str,
    hint_field_name1: &str,
    hint_field_name2: &str,
    hint_options1: *mut u8,
    hint_options2: *mut u8,
    add: bool,
) -> u64 {
    let field_airgroupvalue = CString::new(hint_field_airgroupvalue).unwrap();
    let field_name1 = CString::new(hint_field_name1).unwrap();
    let field_name2: CString = CString::new(hint_field_name2).unwrap();

    unsafe {
        update_airgroupvalue(
            p_setup_ctx,
            p_steps_params as *mut std::os::raw::c_void,
            hint_id,
            field_airgroupvalue.as_ptr() as *mut std::os::raw::c_char,
            field_name1.as_ptr() as *mut std::os::raw::c_char,
            field_name2.as_ptr() as *mut std::os::raw::c_char,
            hint_options1 as *mut std::os::raw::c_void,
            hint_options2 as *mut std::os::raw::c_void,
            add,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn set_hint_field_c(
    p_setup_ctx: *mut c_void,
    p_params: *mut u8,
    values: *mut u8,
    hint_id: u64,
    hint_field_name: &str,
) -> u64 {
    unsafe {
        let field_name = CString::new(hint_field_name).unwrap();
        set_hint_field(
            p_setup_ctx,
            p_params as *mut std::os::raw::c_void,
            values as *mut std::os::raw::c_void,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_id_c(p_setup_ctx: *mut c_void, hint_id: u64, hint_field_name: &str) -> u64 {
    unsafe {
        let field_name = CString::new(hint_field_name).unwrap();
        get_hint_id(p_setup_ctx, hint_id, field_name.as_ptr() as *mut std::os::raw::c_char)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_new_c(p_setup_ctx: *mut c_void, p_const_tree: *mut u8) -> *mut c_void {
    unsafe { starks_new(p_setup_ctx, p_const_tree as *mut std::os::raw::c_void) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_free_c(p_stark: *mut c_void) {
    unsafe {
        starks_free(p_stark);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn free_provers_c(n_proofs: u64, p_starks: *mut *mut c_void, p_proofs: *mut *mut c_void, background: bool) {
    unsafe {
        proofs_free(n_proofs, p_starks, p_proofs, background);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn treesGL_get_root_c(pStark: *mut c_void, index: u64, root: *mut u8) {
    unsafe {
        treesGL_get_root(pStark, index, root as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn treesGL_set_root_c(pStark: *mut c_void, index: u64, pProof: *mut c_void) {
    unsafe {
        treesGL_set_root(pStark, index, pProof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_xdivxsub_c(p_stark: *mut c_void, xi_challenge: *mut c_void, xdivxsub: *mut u8) {
    unsafe {
        calculate_xdivxsub(p_stark, xi_challenge, xdivxsub as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_fri_pol_c(p_stark_info: *mut c_void, buffer: *mut u8) -> *mut c_void {
    unsafe { get_fri_pol(p_stark_info, buffer as *mut std::os::raw::c_void) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_fri_polynomial_c(p_starks: *mut c_void, p_steps_params: *mut u8) {
    unsafe {
        calculate_fri_polynomial(p_starks, p_steps_params as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_quotient_polynomial_c(p_starks: *mut c_void, p_steps_params: *mut u8) {
    unsafe {
        calculate_quotient_polynomial(p_starks, p_steps_params as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_impols_expressions_c(p_starks: *mut c_void, step: u64, p_steps_params: *mut u8) {
    unsafe {
        calculate_impols_expressions(p_starks, step, p_steps_params as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn extend_and_merkelize_custom_commit_c(
    p_starks: *mut c_void,
    commit_id: u64,
    step: u64,
    buffer: *mut u8,
    buffer_ext: *mut u8,
    p_proof: *mut c_void,
    p_buff_helper: *mut u8,
    buffer_file: &str,
) {
    let buffer_file_name = CString::new(buffer_file).unwrap();
    unsafe {
        extend_and_merkelize_custom_commit(
            p_starks,
            commit_id,
            step,
            buffer as *mut std::os::raw::c_void,
            buffer_ext as *mut std::os::raw::c_void,
            p_proof,
            p_buff_helper as *mut std::os::raw::c_void,
            buffer_file_name.as_ptr() as *mut std::os::raw::c_char,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn load_custom_commit_c(
    p_starks: *mut c_void,
    commit_id: u64,
    step: u64,
    buffer: *mut u8,
    buffer_ext: *mut u8,
    p_proof: *mut c_void,
    buffer_file: &str,
) {
    let buffer_file_name = CString::new(buffer_file).unwrap();
    unsafe {
        load_custom_commit(
            p_starks,
            commit_id,
            step,
            buffer as *mut std::os::raw::c_void,
            buffer_ext as *mut std::os::raw::c_void,
            p_proof,
            buffer_file_name.as_ptr() as *mut std::os::raw::c_char,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_stage_c(
    p_starks: *mut c_void,
    element_type: u32,
    step: u64,
    witness: *mut u8,
    buffer: *mut u8,
    p_proof: *mut c_void,
    p_buff_helper: *mut u8,
) {
    unsafe {
        commit_stage(
            p_starks,
            element_type,
            step,
            witness as *mut std::os::raw::c_void,
            buffer as *mut std::os::raw::c_void,
            p_proof,
            p_buff_helper as *mut std::os::raw::c_void,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_lev_c(p_stark: *mut c_void, xi_challenge: *mut c_void, lev: *mut u8) {
    unsafe {
        compute_lev(p_stark, xi_challenge, lev as *mut std::os::raw::c_void);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_evals_c(p_stark: *mut c_void, params: *mut u8, lev: *mut u8, pProof: *mut c_void) {
    unsafe {
        compute_evals(p_stark, params as *mut std::os::raw::c_void, lev as *mut std::os::raw::c_void, pProof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_folding_c(
    step: u64,
    buffer: *mut u8,
    challenge: *mut u8,
    n_bits_ext: u64,
    prev_bits: u64,
    current_bits: u64,
) {
    unsafe {
        compute_fri_folding(
            step,
            buffer as *mut std::os::raw::c_void,
            challenge as *mut std::os::raw::c_void,
            n_bits_ext,
            prev_bits,
            current_bits,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_merkelize_c(
    p_starks: *mut c_void,
    p_proof: *mut c_void,
    step: u64,
    buffer: *mut u8,
    current_bits: u64,
    next_bits: u64,
) {
    unsafe {
        compute_fri_merkelize(p_starks, p_proof, step, buffer as *mut std::os::raw::c_void, current_bits, next_bits);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_queries_c(
    p_stark: *mut c_void,
    p_proof: *mut c_void,
    p_fri_queries: *mut u64,
    n_queries: u64,
    n_trees: u64,
) {
    unsafe {
        compute_queries(p_stark, p_proof, p_fri_queries, n_queries, n_trees);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn compute_fri_queries_c(
    p_starks: *mut c_void,
    p_proof: *mut c_void,
    p_fri_queries: *mut u64,
    n_queries: u64,
    step: u64,
    current_bits: u64,
) {
    unsafe {
        compute_fri_queries(p_starks, p_proof, p_fri_queries, n_queries, step, current_bits);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn set_fri_final_pol_c(p_proof: *mut c_void, buffer: *mut u8, n_bits: u64) {
    unsafe {
        set_fri_final_pol(p_proof, buffer as *mut std::os::raw::c_void, n_bits);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn calculate_hash_c(pStarks: *mut c_void, pHhash: *mut u8, pBuffer: *mut u8, nElements: u64) {
    unsafe {
        calculate_hash(pStarks, pHhash as *mut std::os::raw::c_void, pBuffer as *mut std::os::raw::c_void, nElements);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_new_c(element_type: u32, arity: u64, custom: bool) -> *mut c_void {
    unsafe { transcript_new(element_type, arity, custom) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn transcript_add_c(p_transcript: *mut c_void, p_input: *mut u8, size: u64) {
    unsafe {
        transcript_add(p_transcript, p_input as *mut std::os::raw::c_void, size);
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
pub fn get_n_constraints_c(p_setup: *mut c_void) -> u64 {
    unsafe { get_n_constraints(p_setup) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_constraints_lines_sizes_c(p_setup: *mut c_void, constraints_sizes: *mut u64) {
    unsafe {
        get_constraints_lines_sizes(p_setup, constraints_sizes);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_constraints_lines_c(p_setup: *mut c_void, constraints_lines: *mut *mut u8) {
    unsafe {
        get_constraints_lines(p_setup, constraints_lines);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn verify_constraints_c(p_setup: *mut c_void, p_steps_params: *mut u8, constraints_info: *mut c_void) {
    unsafe {
        verify_constraints(p_setup, p_steps_params as *mut std::os::raw::c_void, constraints_info);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_n_global_constraints_c(p_global_constraints_bin: *mut c_void) -> u64 {
    unsafe { get_n_global_constraints(p_global_constraints_bin) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_global_constraints_lines_sizes_c(p_global_constraints_bin: *mut c_void, global_constraints_sizes: *mut u64) {
    unsafe {
        get_global_constraints_lines_sizes(p_global_constraints_bin, global_constraints_sizes);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_global_constraints_lines_c(p_global_constraints_bin: *mut c_void, global_constraints_lines: *mut *mut u8) {
    unsafe {
        get_global_constraints_lines(p_global_constraints_bin, global_constraints_lines);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn verify_global_constraints_c(
    global_info_file: &str,
    p_global_constraints_bin: *mut c_void,
    publics: *mut u8,
    challenges: *mut u8,
    proof_values: *mut u8,
    airgroupvalues: *mut *mut u8,
    global_constraints_info: *mut c_void,
) {
    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        verify_global_constraints(
            global_info_file_ptr,
            p_global_constraints_bin,
            publics as *mut std::os::raw::c_void,
            challenges as *mut std::os::raw::c_void,
            proof_values as *mut std::os::raw::c_void,
            airgroupvalues as *mut *mut std::os::raw::c_void,
            global_constraints_info,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn get_hint_field_global_constraints_c(
    global_info_file: &str,
    p_global_constraints_bin: *mut c_void,
    hint_field_values: *mut c_void,
    publics: *mut u8,
    challenges: *mut u8,
    proof_values: *mut u8,
    airgroupvalues: *mut *mut u8,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) {
    let field_name = CString::new(hint_field_name).unwrap();

    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        get_hint_field_global_constraints(
            global_info_file_ptr,
            p_global_constraints_bin,
            hint_field_values,
            publics as *mut std::os::raw::c_void,
            challenges as *mut std::os::raw::c_void,
            proof_values as *mut std::os::raw::c_void,
            airgroupvalues as *mut *mut std::os::raw::c_void,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
            print_expression,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_global_constraints_values_c(
    p_global_constraints_bin: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
) -> u64 {
    let field_name = CString::new(hint_field_name).unwrap();
    unsafe {
        get_hint_field_global_constraints_values(
            p_global_constraints_bin,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_hint_field_global_constraints_sizes_c(
    global_info_file: &str,
    p_global_constraints_bin: *mut c_void,
    hint_field_values: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
    print_expression: bool,
) {
    let field_name = CString::new(hint_field_name).unwrap();

    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        get_hint_field_global_constraints_sizes(
            global_info_file_ptr,
            p_global_constraints_bin,
            hint_field_values,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
            print_expression,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn set_hint_field_global_constraints_c(
    global_info_file: &str,
    p_global_constraints_bin: *mut c_void,
    proof_values: *mut u8,
    values: *mut u8,
    hint_id: u64,
    hint_field_name: &str,
) -> u64 {
    let field_name = CString::new(hint_field_name).unwrap();

    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        set_hint_field_global_constraints(
            global_info_file_ptr,
            p_global_constraints_bin,
            proof_values as *mut std::os::raw::c_void,
            values as *mut std::os::raw::c_void,
            hint_id,
            field_name.as_ptr() as *mut std::os::raw::c_char,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn print_row_c(p_setup_ctx: *mut c_void, buffer: *mut u8, stage: u64, row: u64) {
    unsafe {
        print_row(p_setup_ctx, buffer as *mut std::os::raw::c_void, stage, row);
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn gen_recursive_proof_c(
    p_setup_ctx: *mut c_void,
    p_witness: *mut u8,
    p_aux_trace: *mut u8,
    p_const_pols: *mut u8,
    p_const_tree: *mut u8,
    p_public_inputs: *mut u8,
    proof_file: &str,
    global_info_file: &str,
    airgroup_id: u64,
    vadcop: bool,
) -> *mut c_void {
    let proof_file_name = CString::new(proof_file).unwrap();
    let proof_file_ptr = proof_file_name.as_ptr() as *mut std::os::raw::c_char;

    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        gen_recursive_proof(
            p_setup_ctx,
            global_info_file_ptr,
            airgroup_id,
            p_witness as *mut std::os::raw::c_void,
            p_aux_trace as *mut std::os::raw::c_void,
            p_const_pols as *mut std::os::raw::c_void,
            p_const_tree as *mut std::os::raw::c_void,
            p_public_inputs as *mut std::os::raw::c_void,
            proof_file_ptr,
            vadcop,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_zkin_ptr_c(zkin_file: &str) -> *mut c_void {
    let zkin_file_name = CString::new(zkin_file).unwrap();
    let zkin_file_ptr = zkin_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe { get_zkin_ptr(zkin_file_ptr) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn add_recursive2_verkey_c(p_zkin: *mut c_void, recursive2_verkey: &str) -> *mut c_void {
    let recursive2_verkey_name = CString::new(recursive2_verkey).unwrap();
    let recursive2_verkey_ptr = recursive2_verkey_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe { add_recursive2_verkey(p_zkin, recursive2_verkey_ptr) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn join_zkin_final_c(
    p_publics: *mut u8,
    p_proof_values: *mut u8,
    p_challenges: *mut u8,
    global_info_file: &str,
    zkin_recursive2: *mut *mut c_void,
    stark_info_recursive2: *mut *mut c_void,
) -> *mut c_void {
    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        join_zkin_final(
            p_publics as *mut std::os::raw::c_void,
            p_proof_values as *mut std::os::raw::c_void,
            p_challenges as *mut std::os::raw::c_void,
            global_info_file_ptr,
            zkin_recursive2,
            stark_info_recursive2,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn join_zkin_recursive2_c(
    airgroup_id: u64,
    p_publics: *mut u8,
    p_challenges: *mut u8,
    global_info_file: &str,
    zkin1: *mut c_void,
    zkin2: *mut c_void,
    stark_info_recursive2: *mut c_void,
) -> *mut c_void {
    let global_info_file_name = CString::new(global_info_file).unwrap();
    let global_info_file_ptr = global_info_file_name.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        join_zkin_recursive2(
            global_info_file_ptr,
            airgroup_id,
            p_publics as *mut std::os::raw::c_void,
            p_challenges as *mut std::os::raw::c_void,
            zkin1,
            zkin2,
            stark_info_recursive2,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_serialized_proof_c(zkin: *mut c_void) -> (*mut std::os::raw::c_char, u64) {
    unsafe {
        let size: Box<u64> = Box::new(0);
        let size_ptr: *mut u64 = Box::into_raw(size);
        let ptr = get_serialized_proof(zkin, size_ptr);
        let size: Box<u64> = Box::from_raw(size_ptr);
        (ptr, *size)
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn deserialize_zkin_proof_c(zkin_cstr: *mut std::os::raw::c_char) -> *mut c_void {
    unsafe { deserialize_zkin_proof(zkin_cstr) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn get_zkin_proof_c(zkin: *mut std::os::raw::c_char) -> *mut c_void {
    unsafe { get_zkin_proof(zkin) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn zkin_proof_free_c(p_zkin_proof: *mut c_void) {
    unsafe {
        zkin_proof_free(p_zkin_proof);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn serialized_proof_free_c(zkin_cstr: *mut std::os::raw::c_char) {
    unsafe {
        serialized_proof_free(zkin_cstr);
    }
}

#[cfg(not(feature = "no_lib_link"))]
#[allow(clippy::too_many_arguments)]
pub fn get_committed_pols_c(
    circomWitness: *mut u8,
    execFile: *const i8,
    witness: *mut u8,
    pPublics: *mut u8,
    sizeWitness: u64,
    N: u64,
    nPublics: u64,
    nCols: u64,
) {
    unsafe {
        get_committed_pols(
            circomWitness as *mut std::os::raw::c_void,
            execFile as *mut std::os::raw::c_char,
            witness as *mut std::os::raw::c_void,
            pPublics as *mut std::os::raw::c_void,
            sizeWitness,
            N,
            nPublics,
            nCols,
        );
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn gen_final_snark_proof_c(circomWitnessFinal: *mut u8, zkeyFile: &str, outputDir: &str) {
    let zkey_file_name = CString::new(zkeyFile).unwrap();
    let zkey_file_ptr = zkey_file_name.as_ptr() as *mut std::os::raw::c_char;

    let output_dir_name = CString::new(outputDir).unwrap();
    let output_dir_ptr = output_dir_name.as_ptr() as *mut std::os::raw::c_char;
    unsafe {
        gen_final_snark_proof(circomWitnessFinal as *mut std::os::raw::c_void, zkey_file_ptr, output_dir_ptr);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn set_log_level_c(level: u64) {
    unsafe {
        setLogLevel(level);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn stark_verify_c(
    verkey: &str,
    p_proof: *mut c_void,
    p_stark_info: *mut c_void,
    p_expressions_bin: *mut c_void,
    p_publics: *mut u8,
    p_proof_values: *mut u8,
    p_challenges: *mut u8,
) -> bool {
    let verkey_file = CString::new(verkey).unwrap();
    let verkey_file_ptr = verkey_file.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        stark_verify(
            p_proof,
            p_stark_info,
            p_expressions_bin,
            verkey_file_ptr,
            p_publics as *mut c_void,
            p_proof_values as *mut c_void,
            p_challenges as *mut c_void,
        )
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn save_file_c(p_buffer: *mut u8, buffer_size: u64, p_publics: *mut u8, publics_size: u64, file_name: &str) {
    let file = CString::new(file_name).unwrap();
    let file_ptr = file.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        save_to_file(p_buffer as *mut c_void, buffer_size, p_publics as *mut c_void, publics_size, file_ptr);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn read_from_file_c(p_buffer: *mut u8, buffer_size: u64, p_publics: *mut u8, publics_size: u64, file_name: &str) {
    let file = CString::new(file_name).unwrap();
    let file_ptr = file.as_ptr() as *mut std::os::raw::c_char;

    unsafe {
        read_from_file(p_buffer as *mut c_void, buffer_size, p_publics as *mut c_void, publics_size, file_ptr);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn create_buffer_c(size: u64) -> *mut c_void {
    unsafe { create_buffer(size) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn free_buffer_c(buffer: *mut u8) {
    unsafe {
        free_buffer(buffer as *mut c_void);
    }
}

// ------------------------
// MOCK METHODS FOR TESTING
// ------------------------
#[cfg(feature = "no_lib_link")]
pub fn save_challenges_c(_p_challenges: *mut u8, _global_info_file: &str, _output_dir: &str) {
    trace!("{}: ··· {}", "ffi     ", "save_challenges: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn save_publics_c(_n_publics: u64, _public_inputs: *mut u8, _output_dir: &str) {
    trace!("{}: ··· {}", "ffi     ", "save_publics: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn save_proof_values_c(_proof_values: *mut u8, _global_info_file: &str, _output_dir: &str) {
    trace!("{}: ··· {}", "ffi     ", "save_proof_values: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_new_c(_p_setup_ctx: *mut c_void, _instance_id: u64) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_get_tree_root_c(_p_fri_proof: *mut c_void, _root: *mut u8, _tree_index: u64) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_get_tree_root: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_set_airgroup_values_c(_p_fri_proof: *mut c_void, _p_params: *mut u8) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "fri_proof_set_airgroup_values: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_set_air_values_c(_p_fri_proof: *mut c_void, _p_params: *mut u8) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "fri_proof_set_air_values: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn fri_proof_get_zkinproofs_c(
    _n_proofs: u64,
    _proofs: *mut *mut c_void,
    _fri_proofs: *mut *mut c_void,
    _p_publics: *mut u8,
    _p_proof_values: *mut u8,
    _p_challenges: *mut u8,
    _global_info_file: &str,
    _output_dir_file: &str,
) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "fri_proof_get_zkinproofs: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn fri_proof_get_zkinproof_c(
    _p_fri_proof: *mut c_void,
    _p_publics: *mut u8,
    _p_challenges: *mut u8,
    _p_proof_values: *mut u8,
    _global_info_file: &str,
    _output_dir_file: &str,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_get_zkinproof: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_free_zkinproof_c(_p_fri_proof: *mut c_void) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "fri_proof_free_zkinproof: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn fri_proof_free_c(_p_fri_proof: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "fri_proof_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn free_provers_c(_n_proofs: u64, _p_starks: *mut *mut c_void, _fri_proofs: *mut *mut c_void, _background: bool) {
    trace!("{}: ··· {}", "ffi     ", "proofs_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn stark_info_new_c(_filename: &str, _verify: bool) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "starkinfo_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_map_totaln_c(_p_stark_info: *mut c_void, _recursive: bool) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_map_totaln: This is a mock call because there is no linked library");
    100000000
}

#[cfg(feature = "no_lib_link")]
pub fn get_custom_commit_id_c(_p_stark_info: *mut c_void, _name: &str) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_custom_commit_id: This is a mock call because there is no linked library");
    100000000
}

#[cfg(feature = "no_lib_link")]
pub fn stark_info_free_c(_p_stark_info: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "starkinfo_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn prover_helpers_new_c(_p_stark_info: *mut c_void, _pil1: bool) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "prover_helpers_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn prover_helpers_free_c(_p_prover_helpers: *mut c_void) {}

#[cfg(feature = "no_lib_link")]
pub fn load_const_pols_c(_pConstPolsAddress: *mut u8, _const_filename: &str, _const_size: u64) {
    trace!("{}: ··· {}", "ffi     ", "load_const_pols: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_const_tree_size_c(_pStarkInfo: *mut c_void) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_const_tree_size: This is a mock call because there is no linked library");
    1000000
}

#[cfg(feature = "no_lib_link")]
pub fn get_const_size_c(_pStarkInfo: *mut c_void) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_const_size: This is a mock call because there is no linked library");
    1000000
}

#[cfg(feature = "no_lib_link")]
pub fn load_const_tree_c(_pConstPolsTreeAddress: *mut u8, _tree_filename: &str, _const_tree_size: u64) {
    trace!("{}: ··· {}", "ffi     ", "load_const_tree: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_const_tree_c(
    _pStarkInfo: *mut c_void,
    _pConstPols: *mut u8,
    _pConstPolsTreeAddress: *mut u8,
    _tree_filename: &str,
) {
    trace!("{}: ··· {}", "ffi     ", "calculate_const_tree: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn expressions_bin_new_c(_filename: &str, _global: bool, _verify: bool) -> *mut c_void {
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn expressions_bin_free_c(_p_expressions_bin: *mut c_void) {}

#[cfg(feature = "no_lib_link")]
pub fn n_hint_ids_by_name_c(_p_expressions_bin: *mut c_void, _hint_name: &str) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "n_hint_ids_by_name: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_ids_by_name_c(_p_expressions_bin: *mut c_void, _hint_ids: *mut u64, _hint_name: &str) {
    trace!("{}: ··· {}", "ffi     ", "get_hint_ids_by_name: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_c(
    _p_setup_ctx: *mut c_void,
    _p_steps_params: *mut u8,
    _hint_field_values: *mut c_void,
    _hint_id: u64,
    _hint_field_name: &str,
    _hint_options: *mut u8,
) {
    trace!("{}: ··· {}", "ffi     ", "get_hint_field: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_sizes_c(
    _p_setup_ctx: *mut c_void,
    _hint_field_values: *mut c_void,
    _hint_id: u64,
    _hint_field_name: &str,
    _hint_options: *mut u8,
) {
    trace!("{}: ··· {}", "ffi     ", "get_hint_field_sizes: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_values_c(_p_setup_ctx: *mut c_void, _hint_id: u64, _hint_field_name: &str) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_hint_field: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn mul_hint_fields_c(
    _p_setup_ctx: *mut c_void,
    _p_steps_params: *mut u8,
    _hint_id: u64,
    _hint_field_dest: &str,
    _hint_field_name1: &str,
    _hint_field_name2: &str,
    _hint_options1: *mut u8,
    _hint_options2: *mut u8,
) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "mul_hint_fields: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn acc_hint_field_c(
    _p_setup_ctx: *mut c_void,
    _p_steps_params: *mut u8,
    _hint_id: u64,
    _hint_field_dest: &str,
    _hint_field_airgroupvalue: &str,
    _hint_field_name: &str,
    _add: bool,
) {
    trace!("{}: ··· {}", "ffi     ", "acc_hint_fields: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn acc_mul_hint_fields_c(
    _p_setup_ctx: *mut c_void,
    _p_steps_params: *mut u8,
    _hint_id: u64,
    _hint_field_dest: &str,
    _hint_field_airgroupvalue: &str,
    _hint_field_name1: &str,
    _hint_field_name2: &str,
    _hint_options1: *mut u8,
    _hint_options2: *mut u8,
    _add: bool,
) {
    trace!("{}: ··· {}", "ffi     ", "acc_mul_hint_fields: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn update_airgroupvalue_c(
    _p_setup_ctx: *mut c_void,
    _p_steps_params: *mut u8,
    _hint_id: u64,
    _hint_field_airgroupvalue: &str,
    _hint_field_name1: &str,
    _hint_field_name2: &str,
    _hint_options1: *mut u8,
    _hint_options2: *mut u8,
    _add: bool,
) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "update_airgroupvalue: This is a mock call because there is no linked library");
    10000
}

#[cfg(feature = "no_lib_link")]
pub fn set_hint_field_c(
    _p_setup_ctx: *mut c_void,
    _p_params: *mut u8,
    _values: *mut u8,
    _hint_id: u64,
    _hint_field_name: &str,
) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "set_hint_field: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_id_c(_p_setup_ctx: *mut c_void, _hint_id: u64, _hint_field_name: &str) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_hint_field_id: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn starks_new_c(_p_setup: *mut c_void, _p_const_tree: *mut u8) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn starks_free_c(_p_stark: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn treesGL_get_root_c(_pStark: *mut c_void, _index: u64, _root: *mut u8) {
    trace!("{}: ··· {}", "ffi     ", "treesGL_get_root: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn treesGL_set_root_c(_pStark: *mut c_void, _index: u64, _pProof: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "treesGL_set_root: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_fri_polynomial_c(_p_starks: *mut c_void, _p_steps_params: *mut u8) {
    trace!("mckzkevm: ··· {}", "calculate_fri_polynomial: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_quotient_polynomial_c(_p_starks: *mut c_void, _p_steps_params: *mut u8) {
    trace!("mckzkevm: ··· {}", "calculate_quotient_polynomial: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_impols_expressions_c(_p_starks: *mut c_void, _step: u64, _p_steps_params: *mut u8) {
    trace!(
        "{}: ··· {}",
        "mckzkevm",
        "calculate_impols_expression: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn extend_and_merkelize_custom_commit_c(
    _p_starks: *mut c_void,
    _commit_id: u64,
    _step: u64,
    _buffer: *mut u8,
    _buffer_ext: *mut u8,
    _p_proof: *mut c_void,
    _p_buff_helper: *mut u8,
    _tree_file: &str,
) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "extend_and_merkelize_custom_commit: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn load_custom_commit_c(
    _p_starks: *mut c_void,
    _commit_id: u64,
    _step: u64,
    _buffer: *mut u8,
    _buffer_ext: *mut u8,
    _p_proof: *mut c_void,
    _tree_file: &str,
) {
    trace!("{}: ··· {}", "ffi     ", "load_custom_commit: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn commit_stage_c(
    _p_starks: *mut c_void,
    _element_type: u32,
    _step: u64,
    _witness: *mut u8,
    _buffer: *mut u8,
    _p_proof: *mut c_void,
    _p_buff_helper: *mut u8,
) {
    trace!("{}: ··· {}", "ffi     ", "commit_stage: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_lev_c(_p_stark: *mut c_void, _xi_challenge: *mut c_void, _lev: *mut u8) {
    trace!("{}: ··· {}", "ffi     ", "compute_lev: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_evals_c(_p_stark: *mut c_void, _params: *mut u8, _lev: *mut u8, _pProof: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "compute_evals: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_xdivxsub_c(_p_stark: *mut c_void, _xi_challenge: *mut c_void, _buffer: *mut u8) {
    trace!("{}: ··· {}", "ffi     ", "calculate_xdivxsub: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_fri_pol_c(_p_stark_info: *mut c_void, _buffer: *mut u8) -> *mut c_void {
    trace!("ffi     : ··· {}", "get_fri_pol: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_folding_c(
    _step: u64,
    _buffer: *mut u8,
    _challenge: *mut u8,
    _n_bits_ext: u64,
    _prev_bits: u64,
    _current_bits: u64,
) {
    trace!("{}: ··· {}", "ffi     ", "compute_fri_folding: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_merkelize_c(
    _p_starks: *mut c_void,
    _p_proof: *mut c_void,
    _step: u64,
    _buffer: *mut u8,
    _current_bits: u64,
    _next_bits: u64,
) {
    trace!("{}: ··· {}", "ffi     ", "compute_fri_merkelize: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_queries_c(
    _p_stark: *mut c_void,
    _p_proof: *mut c_void,
    _p_fri_queries: *mut u64,
    _n_queries: u64,
    _n_trees: u64,
) {
    trace!("{}: ··· {}", "ffi     ", "compute_queries: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn compute_fri_queries_c(
    _p_starks: *mut c_void,
    _p_proof: *mut c_void,
    _p_fri_queries: *mut u64,
    _n_queries: u64,
    _step: u64,
    _current_bits: u64,
) {
    trace!("{}: ··· {}", "ffi     ", "compute_fri_queries: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn set_fri_final_pol_c(_p_proof: *mut c_void, _buffer: *mut u8, _n_bits: u64) {
    trace!("{}: ··· {}", "ffi     ", "set_fri_final_pol: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn calculate_hash_c(_pStarks: *mut c_void, _pHhash: *mut u8, _pBuffer: *mut u8, _nElements: u64) {
    trace!("{}: ··· {}", "ffi     ", "calculate_hash: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_new_c(_element_type: u32, _arity: u64, _custom: bool) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "transcript_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn transcript_add_c(_p_transcript: *mut c_void, _p_input: *mut u8, _size: u64) {
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
pub fn get_n_constraints_c(_p_setup: *mut c_void) -> u64 {
    trace!("{}: ··· {}", "ffi     ", "get_n_constraints: This is a mock call because there is no linked library");
    0
}

#[cfg(feature = "no_lib_link")]
pub fn get_constraints_lines_sizes_c(_p_setup: *mut c_void, _constraints_sizes: *mut u64) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_constraints_lines_sizes: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn get_constraints_lines_c(_p_setup: *mut c_void, _constraints_lines: *mut *mut u8) {
    trace!("{}: ··· {}", "ffi     ", "get_constraints_lines: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn verify_constraints_c(_p_setup: *mut c_void, _p_steps_params: *mut u8, _constraints_info: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "verify_constraints: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn get_n_global_constraints_c(_p_global_constraints_bin: *mut c_void) -> u64 {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_n_global_constraints: This is a mock call because there is no linked library"
    );
    0
}

#[cfg(feature = "no_lib_link")]
pub fn get_global_constraints_lines_sizes_c(
    _p_global_constraints_bin: *mut c_void,
    _global_constraints_sizes: *mut u64,
) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_global_constraints_lines_sizes: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn get_global_constraints_lines_c(_p_global_constraints_bin: *mut c_void, _global_constraints_lines: *mut *mut u8) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_global_constraints_lines: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn verify_global_constraints_c(
    _global_info_file: &str,
    _p_global_constraints_bin: *mut c_void,
    _publics: *mut u8,
    _challenges: *mut u8,
    _proof_values: *mut u8,
    _airgroupvalues: *mut *mut u8,
    _global_constraints_info: *mut c_void,
) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "verify_global_constraints: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn get_hint_field_global_constraints_c(
    _global_info_file: &str,
    _p_global_constraints_bin: *mut c_void,
    _hint_field_values: *mut c_void,
    _publics: *mut u8,
    _challenges: *mut u8,
    _proof_values: *mut u8,
    _airgroupvalues: *mut *mut u8,
    _hint_id: u64,
    _hint_field_name: &str,
    _print_expression: bool,
) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_hint_field_global_constraints: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_global_constraints_values_c(
    _p_global_constraints_bin: *mut c_void,
    _hint_id: u64,
    _hint_field_name: &str,
) -> u64 {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_hint_field_global_constraints_values: This is a mock call because there is no linked library"
    );
    0
}

#[cfg(feature = "no_lib_link")]
pub fn get_hint_field_global_constraints_sizes_c(
    _global_info_file: &str,
    _p_global_constraints_bin: *mut c_void,
    _hint_field_values: *mut c_void,
    _hint_id: u64,
    _hint_field_name: &str,
    _print_expression: bool,
) {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "get_hint_field_global_constraints_sizes: This is a mock call because there is no linked library"
    );
}

#[cfg(feature = "no_lib_link")]
pub fn set_hint_field_global_constraints_c(
    _global_info_file: &str,
    _p_global_constraints_bin: *mut c_void,
    _proof_values: *mut u8,
    _values: *mut u8,
    _hint_id: u64,
    _hint_field_name: &str,
) -> u64 {
    trace!(
        "{}: ··· {}",
        "ffi     ",
        "set_hint_field_global_constraints: This is a mock call because there is no linked library"
    );
    100000
}

#[cfg(feature = "no_lib_link")]
pub fn print_row_c(_p_setup_ctx: *mut c_void, _buffer: *mut u8, _stage: u64, _row: u64) {
    trace!("{}: ··· {}", "ffi     ", "print_row: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn gen_recursive_proof_c(
    _p_setup_ctx: *mut c_void,
    _p_address: *mut u8,
    _p_aux_trace: *mut u8,
    _p_const_pols: *mut u8,
    _p_const_tree: *mut u8,
    _p_public_inputs: *mut u8,
    _proof_file: &str,
    _global_info_file: &str,
    _airgroup_id: u64,
    _vadcop: bool,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "gen_recursive_proof: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_zkin_ptr_c(_zkin_file: &str) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "get_zkin_ptr: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn add_recursive2_verkey_c(_p_zkin: *mut c_void, _recursive2_verkey: &str) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "add_recursive2_verkey: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn join_zkin_recursive2_c(
    _airgroup_id: u64,
    _p_publics: *mut u8,
    _p_challenges: *mut u8,
    _global_info_file: &str,
    _zkin1: *mut c_void,
    _zkin2: *mut c_void,
    _stark_info_recursive2: *mut c_void,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "join_zkin_recursive2: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn join_zkin_final_c(
    _p_publics: *mut u8,
    _p_proof_values: *mut u8,
    _p_challenges: *mut u8,
    _global_info_file: &str,
    _zkin_recursive2: *mut *mut c_void,
    _stark_info_recursive2: *mut *mut c_void,
) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "join_zkin_final: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_serialized_proof_c(_zkin: *mut c_void) -> (*mut std::os::raw::c_char, u64) {
    trace!("{}: ··· {}", "ffi     ", "get_serialized_proof: This is a mock call because there is no linked library");
    (std::ptr::null_mut(), 0)
}

#[cfg(feature = "no_lib_link")]
pub fn deserialize_zkin_proof_c(_zkin_cstr: *mut std::os::raw::c_char) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "deserialize_zkin_proof: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn get_zkin_proof_c(_zkin_file: *mut std::os::raw::c_char) -> *mut c_void {
    trace!("{}: ··· {}", "ffi     ", "get_zkin_proof: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn zkin_proof_free_c(_p_zkin_proof: *mut c_void) {
    trace!("{}: ··· {}", "ffi     ", "zkin_proof_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn serialized_proof_free_c(_zkin_cstr: *mut std::os::raw::c_char) {
    trace!("{}: ··· {}", "ffi     ", "serialized_proof_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
#[allow(clippy::too_many_arguments)]
pub fn get_committed_pols_c(
    _circomWitness: *mut u8,
    _execFile: *const i8,
    _witness: *mut u8,
    _pPublics: *mut u8,
    _sizeWitness: u64,
    _N: u64,
    _nPublics: u64,
    _nCols: u64,
) {
    trace!("{}: ··· {}", "ffi     ", "get_committed_pols: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn gen_final_snark_proof_c(_circomWitnessFinal: *mut u8, _zkeyFile: &str, _outputDir: &str) {
    trace!("{}: ··· {}", "ffi     ", "gen_final_snark_proof: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn set_log_level_c(_level: u64) {
    trace!("{}: ··· {}", "ffi     ", "set_log_level: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn stark_verify_c(
    _verkey: &str,
    _p_proof: *mut c_void,
    _p_stark_info: *mut c_void,
    _p_expressions_bin: *mut c_void,
    _p_publics: *mut u8,
    _p_proof_values: *mut u8,
    _p_challenges: *mut u8,
) -> bool {
    trace!("{}: ··· {}", "ffi     ", "stark_verify_c: This is a mock call because there is no linked library");
    true
}
