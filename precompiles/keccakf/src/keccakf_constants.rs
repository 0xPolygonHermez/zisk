use precompiles_helpers::{
    KECCAKF_BITS, KECCAKF_CHUNKS, KECCAKF_INPUT_BITS_IN_PARALLEL, KECCAKF_INPUT_SIZE_BITS,
    KECCAKF_OUTPUT_SIZE_BITS, KECCAK_GATE_CONFIG,
};

/// Generic Parameters
pub const INPUT_DATA_SIZE_BITS: usize = KECCAKF_INPUT_SIZE_BITS as usize;
pub const INPUT_DATA_SIZE_BYTES: usize = INPUT_DATA_SIZE_BITS / 8;
pub const OUTPUT_SIZE_BITS: usize = KECCAKF_OUTPUT_SIZE_BITS as usize;
pub const RB: usize = 32;
pub const RC: usize = 2;
pub const BITS_IN_PARALLEL: usize = KECCAKF_INPUT_BITS_IN_PARALLEL as usize;

pub const RB_BLOCKS_TO_PROCESS: usize = INPUT_DATA_SIZE_BITS / RB;

/// Keccakf circuit Parameters
pub const BITS_KECCAKF: usize = KECCAKF_BITS as usize;
pub const CHUNKS_KECCAKF: usize = KECCAKF_CHUNKS as usize;
pub const P2_BITS_KECCAKF: u64 = 1 << BITS_KECCAKF;
pub const P2_CHUNK_BITS_KECCAKF: u64 = 1 << (BITS_KECCAKF * CHUNKS_KECCAKF);
pub const MASK_BITS_KECCAKF: u64 = P2_BITS_KECCAKF - 1;
pub const MASK_CHUNK_BITS_KECCAKF: u64 = P2_CHUNK_BITS_KECCAKF - 1;
pub const NUM_KECCAKF_PER_CIRCUIT: usize = BITS_KECCAKF * CHUNKS_KECCAKF;
pub const RB_SIZE: usize = NUM_KECCAKF_PER_CIRCUIT * RB * RC / BITS_IN_PARALLEL;
pub const BLOCK_SIZE: usize = NUM_KECCAKF_PER_CIRCUIT * 64 / BITS_IN_PARALLEL;
pub const INPUT_SIZE: usize = NUM_KECCAKF_PER_CIRCUIT * INPUT_DATA_SIZE_BITS / BITS_IN_PARALLEL;
pub const OUTPUT_SIZE: usize = NUM_KECCAKF_PER_CIRCUIT * OUTPUT_SIZE_BITS / BITS_IN_PARALLEL;
pub const IN_DATA_BLOCKS: usize = INPUT_DATA_SIZE_BITS / (RB * RC);
pub const OUT_BLOCKS: usize = OUTPUT_SIZE_BITS / (RB * RC);

/// Keccakf Table Parameters
pub const BITS_KECCAKF_TABLE: usize = BITS_KECCAKF;
pub const CHUNKS_KECCAKF_TABLE: usize = 1;
pub const BITS_A: usize = BITS_KECCAKF_TABLE - CHUNKS_KECCAKF_TABLE + 1;
pub const BITS_B: usize = BITS_KECCAKF_TABLE;
pub const P2_BITS_A: u64 = 1 << BITS_A;
pub const P2_BITS_B: u64 = 1 << BITS_B;
pub const P2_BITS_AB: u64 = P2_BITS_A * P2_BITS_B;
pub const MASK_BITS_A: u64 = P2_BITS_A - 1;
pub const MASK_BITS_B: u64 = P2_BITS_B - 1;

/// Circuit parameters
pub const STATE_IN_FIRST_REF: usize = KECCAK_GATE_CONFIG.sin_first_ref as usize;
pub const STATE_IN_GROUP_BY: usize = KECCAK_GATE_CONFIG.sin_ref_group_by as usize;
pub const STATE_IN_NUMBER: usize = KECCAK_GATE_CONFIG.sin_ref_number as usize;
pub const STATE_IN_REF_DISTANCE: usize = KECCAK_GATE_CONFIG.sin_ref_distance as usize;
pub const XOR_GATE_OP: u8 = 0x00;
pub const ANDP_GATE_OP: u8 = 0x01;
