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
pub const FCALL_DIVISION_SHORT_ID: u16 = 16;
pub const FCALL_DIVISION_LONG_ID: u16 = 17;

mod bls12_381_fp2_inv;
mod bls12_381_fp_inv;
mod bls12_381_fp_sqrt;
mod bls12_381_twist;
mod bn254_fp;
mod bn254_fp2;
mod bn254_twist;
mod division_long;
mod division_short;
mod msb_pos_256;
mod msb_pos_384;
mod secp256k1_fn_inv;
mod secp256k1_fp_inv;
mod secp256k1_fp_sqrt;

pub use bls12_381_fp2_inv::*;
pub use bls12_381_fp_inv::*;
pub use bls12_381_fp_sqrt::*;
pub use bls12_381_twist::*;
pub use bn254_fp::*;
pub use bn254_fp2::*;
pub use bn254_twist::*;
pub use division_long::*;
pub use division_short::*;
pub use msb_pos_256::*;
pub use msb_pos_384::*;
pub use secp256k1_fn_inv::*;
pub use secp256k1_fp_inv::*;
pub use secp256k1_fp_sqrt::*;
