//! Software implementations of free-input calls.
//!
//! These are used on native (non-zkVM) targets for testing and hint/trace generation.
//! Each module mirrors the corresponding [`fcalls`](super::fcalls) wrapper and computes the
//! same result using standard Rust arithmetic.

pub mod bigint_div;
pub mod bin_decomp;
pub mod bls12_381;
pub mod bn254;
pub mod msb_pos_256;
pub mod msb_pos_384;
mod proxy;
pub mod secp256k1;
pub mod secp256r1;
pub mod uint256;
mod utils;

pub use proxy::*;
