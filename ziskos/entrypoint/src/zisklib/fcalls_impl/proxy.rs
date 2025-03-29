use super::{secp256k1_fn_inv::*, secp256k1_fp_inv::*, secp256k1_fp_sqrt::*};
use crate::{
    FCALL_BASE_ID, FCALL_SECP256K1_FN_INV_ID, FCALL_SECP256K1_FP_INV_ID, FCALL_SECP256K1_FP_SQRT_ID,
};

pub fn fcall_proxy(
    id: u64,
    params: &[u64],
    results: &mut [u64],
    mem_read: impl Fn(u64) -> u64,
) -> i64 {
    match FCALL_BASE_ID + id as u16 - 1 {
        FCALL_SECP256K1_FN_INV_ID => secp256k1_fn_inv(params, results, mem_read),
        FCALL_SECP256K1_FP_INV_ID => secp256k1_fp_inv(params, results, mem_read),
        FCALL_SECP256K1_FP_SQRT_ID => secp256k1_fp_sqrt(params, results, mem_read),
        _ => panic!("Unsupported fcall ID {}", id),
    }
}
