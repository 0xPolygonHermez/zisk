//! Free-input call (fcall) wrappers.
//!
//! Fcalls provide *unverified hints* from the host to the guest. Unlike syscalls, the VM does
//! not automatically prove correctness—callers must verify the result (e.g. check `a * inv ≡ 1`).
//!
//! On zkVM targets each fcall issues a CSR read; on native targets it delegates to
//! [`fcalls_impl`](super::fcalls_impl).

// fcalls: CSR 0x8C0 + (id >> 5) with immediate id & 31, so 0x8C0 - 0x8DF (1024 ids)

pub const FCALL_SECP256K1_FP_INV_ID: u16 = 1;
pub const FCALL_SECP256K1_FN_INV_ID: u16 = 2;
pub const FCALL_SECP256K1_FP_SQRT_ID: u16 = 3;
pub const FCALL_SECP256K1_GLV_DECOMPOSE_ID: u16 = 4;
pub const FCALL_SECP256R1_FN_INV_ID: u16 = 5;
pub const FCALL_BN254_FP_INV_ID: u16 = 6;
pub const FCALL_BN254_FP2_INV_ID: u16 = 7;
pub const FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID: u16 = 8;
pub const FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID: u16 = 9;
pub const FCALL_BLS12_381_FP_INV_ID: u16 = 10;
pub const FCALL_BLS12_381_FP_SQRT_ID: u16 = 11;
pub const FCALL_BLS12_381_FP2_INV_ID: u16 = 12;
pub const FCALL_BLS12_381_FP2_SQRT_ID: u16 = 13;
pub const FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID: u16 = 14;
pub const FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID: u16 = 15;
pub const FCALL_BIN_DECOMP_ID: u16 = 16;
pub const FCALL_MSB_POS_256_ID: u16 = 17;
pub const FCALL_MSB_POS_384_ID: u16 = 18;
pub const FCALL_UINT256_DIV_ID: u16 = 19;
pub const FCALL_UINT256_INV_ID: u16 = 20;
pub const FCALL_UINT256_INV_MOD_ID: u16 = 21;
pub const FCALL_BIGINT_DIV_ID: u16 = 22;
pub const FCALL_INPUT_READY_ID: u16 = 23;
pub const FCALL_SET_KECCAKF_CACHE_INDEX_ID: u16 = 24;
pub const FCALL_GET_KECCAKF_CACHE_INDEX_ID: u16 = 25;
pub const FCALL_ED25519_FP_SQRT_RATIO_ID: u16 = 26;
/// Returns the 256-bit host randomness seed (`zkvm_random_u64`), as [`RANDOM_SEED_WORDS`]
/// little-endian words. Resolved by the executor, which receives the seed with each execution;
/// never part of the precompile hints.
pub const FCALL_RANDOM_SEED_ID: u16 = 27;
/// Size of the host randomness seed in 64-bit words (256 bits).
pub const RANDOM_SEED_WORDS: usize = 4;

// Fork-join markers. A parallel region always runs, and is proven, as if its tasks ran one after
// another; these fcalls only tell the emulator where the region starts, where each pass through
// its task loop begins and where it ends, so that it can run the tasks at the same time and stitch
// their traces in order. They carry no hint: the emulators resolve them, they never reach
// `Fcall()` and they are never part of the precompile hints.

/// Start of a parallel region.
pub const FCALL_PAR_FORK_ID: u16 = 28;
/// First step of each pass through a region's task loop; always on a chunk boundary.
pub const FCALL_PAR_BOUNDARY_ID: u16 = 29;
/// End of one task, before the padding that takes the loop to the next chunk boundary.
pub const FCALL_PAR_TASK_END_ID: u16 = 30;
/// Returns, as its one result word, how many steps are left in the current chunk after this one,
/// so that the caller can pad up to the next chunk boundary. A free input that only decides how
/// many steps the padding takes, never what the program computes.
pub const FCALL_PAR_PAD_ID: u16 = 31;
/// End of a parallel region.
pub const FCALL_PAR_JOIN_ID: u16 = 32;

/// Returned by [`fcall_get_keccakf_cache_index`] when the executor has not cached the state.
/// Reserved: it cannot be used as a cache index.
pub const KECCAKF_CACHE_INDEX_NOT_FOUND: u64 = u64::MAX;

mod bigint_div;
mod bin_decomp;
mod bls12_381;
mod bn254;
mod ed25519;
mod input;
mod keccakf_cache;
mod msb_pos_256;
mod msb_pos_384;
mod secp256k1;
mod secp256r1;
mod uint256;

pub use bigint_div::*;
pub use bin_decomp::*;
pub use bls12_381::*;
pub use bn254::*;
pub use ed25519::*;
pub use input::*;
pub use keccakf_cache::*;
pub use msb_pos_256::*;
pub use msb_pos_384::*;
pub use secp256k1::*;
pub use secp256r1::*;
pub use uint256::*;
