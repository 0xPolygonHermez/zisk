//! This module defines constants for the Keccakf precompile.

/// Generic Parameters
pub const INPUT_DATA_SIZE_BITS: usize = 1600;
pub const INPUT_DATA_SIZE_BYTES: usize = INPUT_DATA_SIZE_BITS / 8; // 200
pub const RB: usize = 32;
pub const P2_RB: u64 = 1 << RB;
pub const MASK_RB: u64 = P2_RB - 1;
pub const RB_BLOCKS_TO_PROCESS: usize = INPUT_DATA_SIZE_BITS / RB; // 50

/// Keccakf Parameters
pub const BITS_IN_PARALLEL_KECCAKF: usize = 2;
pub const CHUNKS_KECCAKF: usize = 6;
pub const BITS_KECCAKF: usize = 10;
pub const P2_BITS_KECCAKF: u64 = 1 << BITS_KECCAKF;
pub const P2_CHUNK_BITS_KECCAKF: u64 = 1 << (BITS_KECCAKF * CHUNKS_KECCAKF);
pub const MASK_BITS_KECCAKF: u64 = P2_BITS_KECCAKF - 1;
pub const MASK_CHUNK_BITS_KECCAKF: u64 = P2_CHUNK_BITS_KECCAKF - 1;

/// Keccakf Table Parameters
pub const CHUNKS_KECCAKF_TABLE: usize = 1;
pub const BITS_KECCAKF_TABLE: usize = BITS_KECCAKF;
pub const BITS_A: usize = BITS_KECCAKF_TABLE - CHUNKS_KECCAKF_TABLE + 1;
pub const BITS_B: usize = BITS_KECCAKF_TABLE;
pub const P2_BITS_A: u64 = 1 << BITS_A;
pub const P2_BITS_B: u64 = 1 << BITS_B;
pub const P2_BITS_AB: u64 = P2_BITS_A * P2_BITS_B;
pub const MASK_BITS_A: u64 = P2_BITS_A - 1;
pub const MASK_BITS_B: u64 = P2_BITS_B - 1;

/// Circuit parameters
pub const STATE_IN_REF_0: usize = 61;
pub const STATE_IN_GROUP_BY: usize = 2;
pub const STATE_IN_NUMBER: usize = 1600;
pub const STATE_IN_REF_DISTANCE: usize = 60;
pub const STATE_OUT_REF_0: usize = 48_061; // 61 + 1600 * 30;
pub const STATE_OUT_GROUP_BY: usize = 2;
pub const STATE_OUT_NUMBER: usize = 1600;
pub const STATE_OUT_REF_DISTANCE: usize = 60;
pub const XOR_GATE_OP: u8 = 0x00;
pub const ANDP_GATE_OP: u8 = 0x01;
