// fcall 0x860 - 0x8DF (128 fcalls)

pub const FCALL_BASE_ID: u16 = 0x860;
pub const FCALL_SECP256K1_FP_INV_ID: u16 = 0x860;
pub const FCALL_SECP256K1_FN_INV_ID: u16 = 0x861;
pub const FCALL_SECP256K1_FP_SQRT_ID: u16 = 0x862;
pub const FCALL_SECP256K1_COLLISION_ID: u16 = 0x863;

mod secp256k1_fn_inv;
mod secp256k1_fp_inv;
mod secp256k1_fp_sqrt;
pub use secp256k1_fn_inv::*;
pub use secp256k1_fp_inv::*;
pub use secp256k1_fp_sqrt::*;
