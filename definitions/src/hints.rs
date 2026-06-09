//! Hint code constants for ZisK Precompiles stream processing.
//!
//! These `u32` codes identify control operations and built-in precompile hints
//! exchanged as a stream between the guest and the host hint processor. They live
//! in `zisk-definitions` (a dependency-free, `no_std` leaf crate) so that both the
//! guest-side `ziskos` crate and the host-side `zisk-common` crate can share them
//! without `ziskos` pulling in the full prover stack.
//!
//! The richer hint types (`HintCode`, `BuiltInHint`, `PrecompileHint`, ...) that
//! build on these constants live in `zisk-common`, which re-exports everything here.

// === CONTROL CODES ===
pub const CTRL_START: u32 = 0x0000;
pub const CTRL_END: u32 = 0x0001;
pub const CTRL_CANCEL: u32 = 0x0002;
pub const CTRL_ERROR: u32 = 0x0003;

// === INPUT HINT CODES ===
pub const HINT_INPUT: u32 = 0xF0000;

// === BUILT-IN HINT CODES ===
// SHA256 hint codes
pub const HINT_SHA256: u32 = 0x0100;

// BN254 hint codes
pub const HINT_BN254_G1_ADD: u32 = 0x0200;
pub const HINT_BN254_G1_MUL: u32 = 0x0201;
pub const HINT_BN254_PAIRING_CHECK: u32 = 0x0205;

// Secp256k1 hint codes
pub const HINT_SECP256K1_ECRECOVER: u32 = 0x0300;
pub const HINT_SECP256K1_ECDSA_VERIFY: u32 = 0x0301;

// Secp256r1 hint codes
pub const HINT_SECP256R1_ECDSA_VERIFY: u32 = 0x0380;

// BLS12-381 hint codes
pub const HINT_BLS12_381_G1_ADD: u32 = 0x0400;
pub const HINT_BLS12_381_G1_MSM: u32 = 0x0401;
pub const HINT_BLS12_381_G2_ADD: u32 = 0x0405;
pub const HINT_BLS12_381_G2_MSM: u32 = 0x0406;
pub const HINT_BLS12_381_PAIRING_CHECK: u32 = 0x040A;
pub const HINT_BLS12_381_FP_TO_G1: u32 = 0x0410;
pub const HINT_BLS12_381_FP2_TO_G2: u32 = 0x0411;

// Modular exponentiation hint codes
pub const HINT_MODEXP: u32 = 0x0500;
/// 256-bit modular multiplication (EVM MULMOD opcode).
pub const HINT_MULMOD256: u32 = 0x0501;
/// 256-bit modular reduction (`a mod m`).
pub const HINT_REDUCE_MOD256: u32 = 0x0502;
/// 256-bit modular addition (`(a + b) mod m`).
pub const HINT_ADD_MOD256: u32 = 0x0503;
/// 256-bit modular squaring (`a² mod m`).
pub const HINT_SQUARE_MOD256: u32 = 0x0504;
/// 256-bit modular exponentiation (`base^exp mod m`).
pub const HINT_POW_MOD256: u32 = 0x0505;
/// 256-bit modular inverse (`a⁻¹ mod m`).
pub const HINT_INV_MOD256: u32 = 0x0506;

// KZG hint codes
pub const HINT_VERIFY_KZG_PROOF: u32 = 0x0600;

// Keccak256 hint codes
pub const HINT_KECCAK256: u32 = 0x0700;

// Blake2b hint codes
pub const HINT_BLAKE2B_COMPRESS: u32 = 0x0800;

// RIPEMD-160 hint codes
pub const HINT_RIPEMD160: u32 = 0x0900;
