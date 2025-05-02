//! This module defines constants for the Sha256 precompile.

/// Generic Parameters
pub const INPUT_DATA_SIZE_BITS: usize = 768;
pub const INPUT_DATA_SIZE_BYTES: usize = INPUT_DATA_SIZE_BITS / 8; // 96
pub const INPUT_DATA_SIZE_U64: usize = INPUT_DATA_SIZE_BITS / 64; // 12
pub const STATE_SIZE_BITS: usize = 256;
pub const INPUT_SIZE_BITS: usize = 512;
pub const OUTPUT_SIZE_BITS: usize = 256;
pub const RB: usize = 32;
pub const P2_RB: u64 = 1 << RB;
pub const MASK_RB: u64 = P2_RB - 1;
pub const RB_BLOCKS_TO_PROCESS: usize = INPUT_DATA_SIZE_BITS / RB;

/// Sha256 Parameters
pub const BITS_IN_PARALLEL_SHA256F: usize = 2;
pub const CHUNKS_SHA256F: usize = 9;
pub const BITS_SHA256F: usize = 7;
pub const P2_BITS_SHA256F: u64 = 1 << BITS_SHA256F;
pub const P2_CHUNK_BITS_SHA256F: u64 = 1 << (BITS_SHA256F * CHUNKS_SHA256F);
pub const MASK_BITS_SHA256F: u64 = P2_BITS_SHA256F - 1;
pub const MASK_CHUNK_BITS_SHA256F: u64 = P2_CHUNK_BITS_SHA256F - 1;

/// Sha256 Table Parameters
pub const CHUNKS_SHA256F_TABLE: usize = 1;
pub const BITS_SHA256F_TABLE: usize = BITS_SHA256F;
pub const BITS_A: usize = BITS_SHA256F_TABLE - CHUNKS_SHA256F_TABLE + 1;
pub const BITS_B: usize = BITS_SHA256F_TABLE;
pub const BITS_C: usize = BITS_SHA256F_TABLE;
pub const P2_BITS_A: u64 = 1 << BITS_A;
pub const P2_BITS_B: u64 = 1 << BITS_B;
pub const P2_BITS_C: u64 = 1 << BITS_C;
pub const P2_BITS_AB: u64 = P2_BITS_A * P2_BITS_B;
pub const P2_BITS_ABC: u64 = P2_BITS_A * P2_BITS_B * P2_BITS_C;
pub const MASK_BITS_A: u64 = P2_BITS_A - 1;
pub const MASK_BITS_B: u64 = P2_BITS_B - 1;
pub const MASK_BITS_C: u64 = P2_BITS_C - 1;

// /// Circuit parameters
pub const STATE_IN_FIRST_REF: usize = 64;
pub const STATE_IN_REF_DISTANCE: usize = 63;
pub const STATE_OUT_FIRST_REF: usize = 24_256; // 64 + 768 * 63 / 2
pub const STATE_OUT_REF_DISTANCE: usize = 63;
pub const XOR_GATE_OP: u8 = 0x00;
pub const CH_GATE_OP: u8 = 0x01;
pub const MAJ_GATE_OP: u8 = 0x02;
pub const ADD_GATE_OP: u8 = 0x03;
