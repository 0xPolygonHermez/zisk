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

mod bn254_fp;
mod bn254_fp2;
mod bn254_twist;
mod msb_pos_256;
mod secp256k1_fn_inv;
mod secp256k1_fp_inv;
mod secp256k1_fp_sqrt;

pub use bn254_fp::*;
pub use bn254_fp2::*;
pub use bn254_twist::*;
pub use msb_pos_256::*;
pub use secp256k1_fn_inv::*;
pub use secp256k1_fp_inv::*;
pub use secp256k1_fp_sqrt::*;
