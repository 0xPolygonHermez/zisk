#![allow(dead_code)]
//! This module defines constants for the Arith256 precompile.

/// Generic Parameters
pub const ARITH_EQ_ROWS_BY_OP: usize = 16;
pub const ARITH_EQ_CHUNKS: usize = 16;
pub const ARITH_EQ_CHUNK_BITS: usize = 16;
pub const ARITH_EQ_CHUNK_SIZE: usize = 1 << ARITH_EQ_CHUNK_BITS;
pub const ARITH_EQ_CHUNK_BASE_MAX: usize = ARITH_EQ_CHUNK_SIZE - 1;
/// Number of sub-operations the family counts and plans: the 11 little-endian ones and their 11
/// big-endian twins (same equations, operands stored in memory as big-endian integers, proved by
/// the `big_endian: 1` airs).
pub const ARITH_EQ_OP_NUM: usize = 22;
/// Number of little-endian sub-operations; a big-endian op's index is its twin's plus this.
pub const ARITH_EQ_LE_OP_NUM: usize = 11;

pub const SEL_OP_ARITH256: usize = 0;
pub const SEL_OP_ARITH256_MOD: usize = 1;
pub const SEL_OP_SECP256K1_ADD: usize = 2;
pub const SEL_OP_SECP256K1_DBL: usize = 3;
pub const SEL_OP_BN254_CURVE_ADD: usize = 4;
pub const SEL_OP_BN254_CURVE_DBL: usize = 5;
pub const SEL_OP_BN254_COMPLEX_ADD: usize = 6;
pub const SEL_OP_BN254_COMPLEX_SUB: usize = 7;
pub const SEL_OP_BN254_COMPLEX_MUL: usize = 8;
pub const SEL_OP_SECP256R1_ADD: usize = 9;
pub const SEL_OP_SECP256R1_DBL: usize = 10;
pub const SEL_OP_ARITH256_BE: usize = SEL_OP_ARITH256 + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_ARITH256_MOD_BE: usize = SEL_OP_ARITH256_MOD + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_SECP256K1_ADD_BE: usize = SEL_OP_SECP256K1_ADD + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_SECP256K1_DBL_BE: usize = SEL_OP_SECP256K1_DBL + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_BN254_CURVE_ADD_BE: usize = SEL_OP_BN254_CURVE_ADD + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_BN254_CURVE_DBL_BE: usize = SEL_OP_BN254_CURVE_DBL + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_BN254_COMPLEX_ADD_BE: usize = SEL_OP_BN254_COMPLEX_ADD + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_BN254_COMPLEX_SUB_BE: usize = SEL_OP_BN254_COMPLEX_SUB + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_BN254_COMPLEX_MUL_BE: usize = SEL_OP_BN254_COMPLEX_MUL + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_SECP256R1_ADD_BE: usize = SEL_OP_SECP256R1_ADD + ARITH_EQ_LE_OP_NUM;
pub const SEL_OP_SECP256R1_DBL_BE: usize = SEL_OP_SECP256R1_DBL + ARITH_EQ_LE_OP_NUM;

pub const SECP256K1_PRIME_CHUNKS: [i64; 16] = [
    0xFC2F, 0xFFFF, 0xFFFE, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
    0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
];

pub const BN254_PRIME_CHUNKS: [i64; 16] = [
    0xFD47, 0xD87C, 0x8C16, 0x3C20, 0xCA8D, 0x6871, 0x6A91, 0x9781, 0x585D, 0x8181, 0x45B6, 0xB850,
    0xA029, 0xE131, 0x4E72, 0x3064,
];

pub const SECP256R1_PRIME_CHUNKS: [i64; 16] = [
    0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0001, 0x0000, 0xFFFF, 0xFFFF,
];
