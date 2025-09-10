#![allow(dead_code)]
//! This module defines constants for the Arith384 precompile.

/// Generic Parameters
pub const ARITH_EQ_384_BITS: usize = 384;
pub const ARITH_EQ_384_U64S: usize = ARITH_EQ_384_BITS / 64;
pub const ARITH_EQ_384_U64S_DOUBLE: usize = ARITH_EQ_384_U64S * 2;
pub const ARITH_EQ_384_ROWS_BY_OP: usize = 24;
pub const ARITH_EQ_384_CHUNKS: usize = ARITH_EQ_384_ROWS_BY_OP;
pub const ARITH_EQ_384_CHUNKS_DOUBLE: usize = ARITH_EQ_384_CHUNKS * 2;
pub const ARITH_EQ_384_CHUNK_BITS: usize = ARITH_EQ_384_BITS / ARITH_EQ_384_CHUNKS;
pub const ARITH_EQ_384_OP_NUM: usize = 6;
pub const ARITH_EQ_384_MAX_CEQS: usize = 3;

pub const SEL_OP_ARITH384_MOD: usize = 0;
pub const SEL_OP_BLS12_381_CURVE_ADD: usize = 1;
pub const SEL_OP_BLS12_381_CURVE_DBL: usize = 2;
pub const SEL_OP_BLS12_381_COMPLEX_ADD: usize = 3;
pub const SEL_OP_BLS12_381_COMPLEX_SUB: usize = 4;
pub const SEL_OP_BLS12_381_COMPLEX_MUL: usize = 5;

pub const BLS12_381_PRIME_CHUNKS: [i64; 24] = [
    0xAAAB, 0xFFFF, 0xFFFF, 0xB9FE, 0xFFFF, 0xB153, 0xFFFE, 0x1EAB, 0xF624, 0xF6B0, 0xD2A0, 0x6730,
    0x12BF, 0xF385, 0x4B84, 0x6477, 0xACD7, 0x434B, 0xA7B6, 0x4B1B, 0xE69A, 0x397F, 0x11EA, 0x1A01,
];
