#![allow(dead_code)]
//! This module defines constants for the Arith256 precompile.

/// Generic Parameters
pub const ARITH_EQ_ROWS_BY_OP: usize = 16;
pub const ARITH_EQ_CHUNKS: usize = 16;
pub const ARITH_EQ_CHUNK_BITS: usize = 16;
pub const ARITH_EQ_CHUNK_SIZE: usize = 1 << ARITH_EQ_CHUNK_BITS;
pub const ARITH_EQ_CHUNK_BASE_MAX: usize = ARITH_EQ_CHUNK_SIZE - 1;
pub const ARITH_EQ_OP_NUM: usize = 9;

pub const SEL_OP_ARITH256: usize = 0;
pub const SEL_OP_ARITH256_MOD: usize = 1;
pub const SEL_OP_SECP256K1_ADD: usize = 2;
pub const SEL_OP_SECP256K1_DBL: usize = 3;
pub const SEL_OP_BN254_CURVE_ADD: usize = 4;
pub const SEL_OP_BN254_CURVE_DBL: usize = 5;
pub const SEL_OP_BN254_COMPLEX_ADD: usize = 6;
pub const SEL_OP_BN254_COMPLEX_SUB: usize = 7;
pub const SEL_OP_BN254_COMPLEX_MUL: usize = 8;

pub const SECP256K1_PRIME_CHUNKS: [i64; 16] = [
    0xFC2F, 0xFFFF, 0xFFFE, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
    0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
];

pub const BN254_PRIME_CHUNKS: [i64; 16] = [
    0xFD47, 0xD87C, 0x8C16, 0x3C20, 0xCA8D, 0x6871, 0x6A91, 0x9781, 0x585D, 0x8181, 0x45B6, 0xB850,
    0xA029, 0xE131, 0x4E72, 0x3064,
];
