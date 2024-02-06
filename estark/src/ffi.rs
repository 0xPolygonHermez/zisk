#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use goldilocks::Goldilocks;
use super::verification_key::VerificationKey;

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
pub fn zkevm_steps_free_c(pZkevmSteps: *mut ::std::os::raw::c_void) {
    unsafe {
        zkevm_steps_free(pZkevmSteps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn c12a_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { c12a_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn c12a_steps_free_c(pC12aSteps: *mut ::std::os::raw::c_void) {
    unsafe {
        c12a_steps_free(pC12aSteps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn recursive1_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { recursive1_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn recursive1_steps_free_c(pRecursive1Steps: *mut ::std::os::raw::c_void) {
    unsafe {
        recursive1_steps_free(pRecursive1Steps);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn _recursive2_steps_new_c() -> *mut std::os::raw::c_void {
    unsafe { recursive2_steps_new() }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn _recursive2_steps_free_c(pRecursive2Steps: *mut ::std::os::raw::c_void) {
    unsafe {
        recursive2_steps_free(pRecursive2Steps);
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
pub fn fri_proof_free_c(pZkevmSteps: *mut ::std::os::raw::c_void) {
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
pub fn config_free_c(pConfig: *mut ::std::os::raw::c_void) {
    unsafe {
        config_free(pConfig);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn starks_new_c(
    p_config: *mut ::std::os::raw::c_void,
    const_pols: &str,
    map_const_pols_file: bool,
    constants_tree: &str,
    stark_info: &str,
    p_address: *mut ::std::os::raw::c_void,
) -> *mut ::std::os::raw::c_void {
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
    _p_Starks: *mut ::std::os::raw::c_void,
    _p_fri_proof: *mut ::std::os::raw::c_void,
    _p_public_inputs: &Vec<T>,
    _p_verkey: &VerificationKey<Goldilocks>,
    _p_steps: *mut ::std::os::raw::c_void,
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
pub fn starks_free_c(pStarks: *mut ::std::os::raw::c_void) {
    unsafe {
        starks_free(pStarks);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_pols_starks_new_c(
    pAddress: *mut ::std::os::raw::c_void,
    degree: u64,
    nCommitedPols: u64,
) -> *mut std::os::raw::c_void {
    unsafe { commit_pols_starks_new(pAddress, degree, nCommitedPols) }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn commit_pols_starks_free_c(pCommitPolsStarks: *mut ::std::os::raw::c_void) {
    unsafe {
        commit_pols_starks_free(pCommitPolsStarks);
    }
}

#[cfg(not(feature = "no_lib_link"))]
pub fn circom_get_commited_pols_c(
    p_commit_pols_starks: *mut ::std::os::raw::c_void,
    zkevm_ferifier: &str,
    exec_file: &str,
    zkin: *mut ::std::os::raw::c_void,
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
    p_commit_pols_starks: *mut ::std::os::raw::c_void,
    zkevm_ferifier: &str,
    exec_file: &str,
    zkin: *mut ::std::os::raw::c_void,
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
    p_fri_proof: *mut ::std::os::raw::c_void,
    public_inputs: &Vec<T>,
    root_c: &Vec<Goldilocks>,
) -> *mut ::std::os::raw::c_void {
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
pub fn zkevm_steps_free_c(_pZkevmSteps: *mut ::std::os::raw::c_void) {
    println!("zkevm_steps_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn c12a_steps_new_c() -> *mut std::os::raw::c_void {
    println!("c12a_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn c12a_steps_free_c(_pC12aSteps: *mut ::std::os::raw::c_void) {
    println!("c12a_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn recursive1_steps_new_c() -> *mut std::os::raw::c_void {
    println!("recursive1_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn recursive1_steps_free_c(_pRecursive1Steps: *mut ::std::os::raw::c_void) {
    println!("recursive1_steps_free_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn _recursive2_steps_new_c() -> *mut std::os::raw::c_void {
    println!("recursive2_steps_new_c: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn _recursive2_steps_free_c(_pRecursive2Steps: *mut ::std::os::raw::c_void) {
    println!("recursive2_steps_free_c: This is a mock call because there is no linked library");
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
pub fn fri_proof_free_c(_pZkevmSteps: *mut ::std::os::raw::c_void) {
    println!("fri_proof_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn config_new_c(config_filename: &str) -> *mut std::os::raw::c_void {
    println!("config_new: This is a mock call because there is no linked library {}", config_filename);
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn config_free_c(_pConfig: *mut ::std::os::raw::c_void) {
    println!("config_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn starks_new_c(
    _p_config: *mut ::std::os::raw::c_void,
    _const_pols: &str,
    _map_const_pols_file: bool,
    _constants_tree: &str,
    _stark_info: &str,
    _p_address: *mut ::std::os::raw::c_void,
) -> *mut ::std::os::raw::c_void {
    println!("starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn starks_genproof_c<T>(
    _p_Starks: *mut ::std::os::raw::c_void,
    _p_fri_proof: *mut ::std::os::raw::c_void,
    _p_public_inputs: &Vec<T>,
    _p_verkey: &VerificationKey<Goldilocks>,
    _p_steps: *mut ::std::os::raw::c_void,
) {
    println!("starks_genproof_c: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn starks_free_c(_pStarks: *mut ::std::os::raw::c_void) {
    println!("starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_starks_new_c(
    _pAddress: *mut ::std::os::raw::c_void,
    _degree: u64,
    _nCommitedPols: u64,
) -> *mut std::os::raw::c_void {
    println!("commit_pols_starks_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}

#[cfg(feature = "no_lib_link")]
pub fn commit_pols_starks_free_c(_pCommitPolsStarks: *mut ::std::os::raw::c_void) {
    println!("commit_pols_starks_free: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn circom_get_commited_pols_c(
    _p_commit_pols_starks: *mut ::std::os::raw::c_void,
    _zkevm_ferifier: &str,
    _exec_file: &str,
    _zkin: *mut ::std::os::raw::c_void,
    _n: u64,
    _n_cols: u64,
) {
    println!("circom_get_commited_pols: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn circom_recursive1_get_commited_pols_c(
    _p_commit_pols_starks: *mut ::std::os::raw::c_void,
    _zkevm_ferifier: &str,
    _exec_file: &str,
    _zkin: *mut ::std::os::raw::c_void,
    _n: u64,
    _n_cols: u64,
) {
    println!("circom_recursive1_get_commited_pols: This is a mock call because there is no linked library");
}

#[cfg(feature = "no_lib_link")]
pub fn zkin_new_c<T>(
    _p_fri_proof: *mut ::std::os::raw::c_void,
    _public_inputs: &Vec<T>,
    _root_c: &Vec<Goldilocks>,
) -> *mut ::std::os::raw::c_void {
    println!("zkin_new: This is a mock call because there is no linked library");
    std::ptr::null_mut()
}
