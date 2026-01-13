// fcall 0x860 - 0x8DF (128 fcalls)

pub const FCALL_SECP256K1_FP_INV_ID: u16 = 1;
pub const FCALL_SECP256K1_FN_INV_ID: u16 = 2;
pub const FCALL_SECP256K1_FP_SQRT_ID: u16 = 3;
pub const FCALL_MSB_POS_256_ID: u16 = 4;
pub const FCALL_SECP256K1_MSM_EDGES_ID: u16 = 5;
pub const FCALL_BN254_FP_INV_ID: u16 = 6;
pub const FCALL_BN254_FP2_INV_ID: u16 = 7;
pub const FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID: u16 = 8;
pub const FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID: u16 = 9;
pub const FCALL_BLS12_381_FP_INV_ID: u16 = 10;
pub const FCALL_BLS12_381_FP_SQRT_ID: u16 = 11;
pub const FCALL_BLS12_381_FP2_INV_ID: u16 = 12;
pub const FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID: u16 = 13;
pub const FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID: u16 = 14;
pub const FCALL_MSB_POS_384_ID: u16 = 15;
pub const FCALL_BIG_INT256_DIV_ID: u16 = 16;
pub const FCALL_BIG_INT_DIV_ID: u16 = 17;
pub const FCALL_BIN_DECOMP_ID: u16 = 18;
pub const FCALL_BLS12_381_FP2_SQRT_ID: u16 = 19;
pub const FCALL_SECP256K1_ECDSA_VERIFY_ID: u16 = 20;

mod big_int256_div;
mod big_int_div;
mod bin_decomp;
mod bls12_381;
mod bn254;
mod msb_pos_256;
mod msb_pos_384;
mod secp256k1;

pub use big_int256_div::*;
pub use big_int_div::*;
pub use bin_decomp::*;
pub use bls12_381::*;
pub use bn254::*;
pub use msb_pos_256::*;
pub use msb_pos_384::*;
pub use secp256k1::*;
