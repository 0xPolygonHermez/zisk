use super::{
    bn254_fp::*, bn254_fp2::*, bn254_twist::*, msb_pos_256::*, secp256k1_fn_inv::*,
    secp256k1_fp_inv::*, secp256k1_fp_sqrt::*,
};
use crate::{
    FCALL_BN254_FP2_INV_ID, FCALL_BN254_FP_INV_ID, FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID,
    FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID, FCALL_MSB_POS_256_ID, FCALL_SECP256K1_FN_INV_ID,
    FCALL_SECP256K1_FP_INV_ID, FCALL_SECP256K1_FP_SQRT_ID,
};

pub fn fcall_proxy(id: u64, params: &[u64], results: &mut [u64]) -> i64 {
    match id as u16 {
        FCALL_SECP256K1_FN_INV_ID => fcall_secp256k1_fn_inv(params, results),
        FCALL_SECP256K1_FP_INV_ID => fcall_secp256k1_fp_inv(params, results),
        FCALL_SECP256K1_FP_SQRT_ID => fcall_secp256k1_fp_sqrt(params, results),
        FCALL_MSB_POS_256_ID => fcall_msb_pos_256(params, results),
        FCALL_BN254_FP_INV_ID => fcall_bn254_fp_inv(params, results),
        FCALL_BN254_FP2_INV_ID => fcall_bn254_fp2_inv(params, results),
        FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID => fcall_bn254_twist_add_line_coeffs(params, results),
        FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID => fcall_bn254_twist_dbl_line_coeffs(params, results),
        _ => panic!("Unsupported fcall ID {id}"),
    }
}
