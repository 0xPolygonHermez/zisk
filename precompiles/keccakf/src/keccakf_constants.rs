//! This module defines constants for the Keccakf precompile.

/// Parameters
pub const CHUNKS: usize = 6;
pub const BITS: usize = 10;
pub const P2_BITS: u64 = 1 << BITS;
const P2_CHUNK_BITS: u64 = 1 << (BITS * CHUNKS);
pub const P2_BITS_SQUARED: u64 = P2_BITS * P2_BITS;
pub const MASK_BITS: u64 = P2_BITS - 1;
pub const MASK_CHUNK_BITS: u64 = P2_CHUNK_BITS - 1;
pub const INPUT_DATA_SIZE_BYTES: u64 = 200; // 1600 bits is the state size

/// Gate types
pub const XOR_GATE_OP: u8 = 0x00;
pub const ANDP_GATE_OP: u8 = 0x01;
