use crate::zisklib::{
    FCALL_BIG_INT256_DIV_ID, FCALL_BIG_INT_DIV_ID, FCALL_BIN_DECOMP_ID, FCALL_BLS12_381_FP2_INV_ID,
    FCALL_BLS12_381_FP2_SQRT_ID, FCALL_BLS12_381_FP_INV_ID, FCALL_BLS12_381_FP_SQRT_ID,
    FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID, FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID,
    FCALL_BN254_FP2_INV_ID, FCALL_BN254_FP_INV_ID, FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID,
    FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID, FCALL_MSB_POS_256_ID, FCALL_MSB_POS_384_ID,
    FCALL_SECP256K1_ECDSA_VERIFY_ID, FCALL_SECP256K1_FN_INV_ID, FCALL_SECP256K1_FP_INV_ID,
    FCALL_SECP256K1_FP_SQRT_ID,
};

use super::{
    big_int256_div::*, big_int_div::*, bin_decomp::*, bls12_381::*, bn254::*, msb_pos_256::*,
    msb_pos_384::*, secp256k1::*,
};

pub fn fcall_proxy(id: u64, params: &[u64], results: &mut [u64]) -> i64 {
    match id as u16 {
        FCALL_SECP256K1_FN_INV_ID => fcall_secp256k1_fn_inv(params, results),
        FCALL_SECP256K1_FP_INV_ID => fcall_secp256k1_fp_inv(params, results),
        FCALL_SECP256K1_FP_SQRT_ID => fcall_secp256k1_fp_sqrt(params, results),
        FCALL_SECP256K1_ECDSA_VERIFY_ID => fcall_secp256k1_ecdsa_verify(params, results),
        FCALL_BN254_FP_INV_ID => fcall_bn254_fp_inv(params, results),
        FCALL_BN254_FP2_INV_ID => fcall_bn254_fp2_inv(params, results),
        FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID => fcall_bn254_twist_add_line_coeffs(params, results),
        FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID => fcall_bn254_twist_dbl_line_coeffs(params, results),
        FCALL_BLS12_381_FP_INV_ID => fcall_bls12_381_fp_inv(params, results),
        FCALL_BLS12_381_FP_SQRT_ID => fcall_bls12_381_fp_sqrt(params, results),
        FCALL_BLS12_381_FP2_INV_ID => fcall_bls12_381_fp2_inv(params, results),
        FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID => {
            fcall_bls12_381_twist_add_line_coeffs(params, results)
        }
        FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID => {
            fcall_bls12_381_twist_dbl_line_coeffs(params, results)
        }
        FCALL_BLS12_381_FP2_SQRT_ID => fcall_bls12_381_fp2_sqrt(params, results),
        FCALL_MSB_POS_256_ID => fcall_msb_pos_256(params, results),
        FCALL_MSB_POS_384_ID => fcall_msb_pos_384(params, results),
        FCALL_BIG_INT256_DIV_ID => fcall_big_int256_div(params, results),
        FCALL_BIG_INT_DIV_ID => fcall_big_int_div(params, results),
        FCALL_BIN_DECOMP_ID => fcall_bin_decomp(params, results),
        _ => panic!("Unsupported fcall ID {id}"),
    }
}
