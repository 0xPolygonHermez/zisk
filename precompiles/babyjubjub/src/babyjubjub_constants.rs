#![allow(dead_code)]
//! Constants for the BabyJubJub (twisted-Edwards) point-add precompile.
//!
//! Curve (circom / circomlib compatible): a*x^2 + y^2 = 1 + d*x^2*y^2 over the
//! BN254 scalar field Fr, with a = 168700, d = 168696.

/// Generic chunk parameters (shared with the AIR-equation generator).
pub const BABYJUBJUB_ROWS_BY_OP: usize = 16;
pub const BABYJUBJUB_CHUNKS: usize = 16;
pub const BABYJUBJUB_CHUNK_BITS: usize = 16;
pub const BABYJUBJUB_CHUNK_SIZE: usize = 1 << BABYJUBJUB_CHUNK_BITS;
pub const BABYJUBJUB_CHUNK_BASE_MAX: usize = BABYJUBJUB_CHUNK_SIZE - 1;

/// This precompile exposes a single operation.
pub const BABYJUBJUB_OP_NUM: usize = 1;
pub const SEL_OP_BABYJUBJUB_ADD: usize = 0;

/// Twisted-Edwards parameters (decimal).
pub const BABYJUBJUB_A: u64 = 168700;
pub const BABYJUBJUB_D: u64 = 168696;

/// BN254 scalar field modulus
/// Fr = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
pub const BABYJUBJUB_PRIME: &str =
    "30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001";

/// `Fr` split into 16 little-endian 16-bit chunks (LSB first).
pub const BABYJUBJUB_PRIME_CHUNKS: [i64; 16] = [
    0x0001, 0xF000, 0xF593, 0x43E1, 0x7091, 0x79B9, 0xE848, 0x2833, 0x585D, 0x8181, 0x45B6, 0xB850,
    0xA029, 0xE131, 0x4E72, 0x3064,
];
