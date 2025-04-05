#![allow(dead_code)]
//! This module defines constants for the Arith256 precompile.

/// Generic Parameters
pub const ARITH_EQ_ROWS_BY_OP: usize = 16;
pub const ARITH_EQ_CHUNKS: usize = 16;
pub const ARITH_EQ_CHUNK_BITS: usize = 16;
pub const ARITH_EQ_CHUNK_SIZE: usize = 1 << ARITH_EQ_CHUNK_BITS;
pub const ARITH_EQ_CHUNK_BASE_MAX: usize = ARITH_EQ_CHUNK_SIZE - 1;

pub const SEL_OP_ARITH256: usize = 0;
pub const SEL_OP_ARITH256_MOD: usize = 1;
pub const SEL_OP_SECP256K1_ADD: usize = 2;
pub const SEL_OP_SECP256K1_DBL: usize = 3;

pub const SECP256K1_PRIME_CHUNKS: [i64; 16] = [
    0xFC2F, 0xFFFF, 0xFFFE, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
    0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
];
