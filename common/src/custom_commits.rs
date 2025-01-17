use proofman_starks_lib_c::{extend_and_merkelize_custom_commit_c, fri_proof_new_c, starks_new_c};
use p3_goldilocks::Goldilocks;

use crate::Setup;

pub fn get_custom_commit_trace(
    commit_id: u64,
    step: u64,
    setup: &Setup,
    buffer: Vec<Goldilocks>,
    buffer_ext: Vec<Goldilocks>,
    buffer_str: &str,
) {
    extend_and_merkelize_custom_commit_c(
        starks_new_c((&setup.p_setup).into(), std::ptr::null_mut()),
        commit_id,
        step,
        buffer.as_ptr() as *mut u8,
        buffer_ext.as_ptr() as *mut u8,
        fri_proof_new_c((&setup.p_setup).into(), 0),
        std::ptr::null_mut(),
        buffer_str,
    );
}
