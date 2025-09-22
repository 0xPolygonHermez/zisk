use precompiles_helpers::{
    KECCAKF_BITS, KECCAKF_CHUNKS, KECCAKF_INPUT_BITS_IN_PARALLEL, KECCAKF_INPUT_SIZE_BITS,
    KECCAKF_OUTPUT_SIZE_BITS, KECCAK_GATE_CONFIG,
};

// Generic Parameters
pub const INPUT_DATA_SIZE_BITS: usize = KECCAKF_INPUT_SIZE_BITS as usize;
pub const OUTPUT_SIZE_BITS: usize = KECCAKF_OUTPUT_SIZE_BITS as usize;
pub const MEM_BITS_IN_PARALLEL: usize = KECCAKF_INPUT_BITS_IN_PARALLEL as usize;

// Keccakf circuit Parameters
pub const BITS_KECCAKF: usize = KECCAKF_BITS as usize;
pub const CHUNKS_KECCAKF: usize = KECCAKF_CHUNKS as usize;
pub const P2_BITS_KECCAKF: u64 = 1 << BITS_KECCAKF;
pub const P2_CHUNK_BITS_KECCAKF: u64 = 1 << (BITS_KECCAKF * CHUNKS_KECCAKF);
pub const MASK_BITS_KECCAKF: u64 = P2_BITS_KECCAKF - 1;
pub const MASK_CHUNK_BITS_KECCAKF: u64 = P2_CHUNK_BITS_KECCAKF - 1;
pub const NUM_KECCAKF_PER_CIRCUIT: usize = BITS_KECCAKF * CHUNKS_KECCAKF;

pub const INPUT_SIZE: usize = INPUT_DATA_SIZE_BITS * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;
pub const OUTPUT_SIZE: usize = OUTPUT_SIZE_BITS * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;

pub const RB: usize = 32;
pub const RC: usize = 2;
pub const MEM_BITS: usize = RB * RC;
pub const IN_BLOCKS: usize = INPUT_DATA_SIZE_BITS / MEM_BITS;
pub const OUT_BLOCKS: usize = OUTPUT_SIZE_BITS / MEM_BITS;
pub const IN_OUT_BLOCKS: usize = IN_BLOCKS + OUT_BLOCKS;

pub const STATE_BITS: usize = 64;
pub const STATE_SIZE: usize = STATE_BITS * NUM_KECCAKF_PER_CIRCUIT / MEM_BITS_IN_PARALLEL;

// Keccakf Table Parameters
pub const BITS_KECCAKF_TABLE: usize = BITS_KECCAKF;
pub const CHUNKS_KECCAKF_TABLE: usize = 1;
pub const BITS_A: usize = BITS_KECCAKF_TABLE - CHUNKS_KECCAKF_TABLE + 1;
pub const BITS_B: usize = BITS_KECCAKF_TABLE;
pub const BITS_C: usize = BITS_KECCAKF_TABLE;
pub const P2_BITS_A: u64 = 1 << BITS_A;
pub const P2_BITS_B: u64 = 1 << BITS_B;
pub const P2_BITS_C: u64 = 1 << BITS_C;
pub const P2_BITS_AB: u64 = P2_BITS_A * P2_BITS_B;
pub const P2_BITS_ABC: u64 = P2_BITS_AB * P2_BITS_C;
pub const MASK_BITS_A: u64 = P2_BITS_A - 1;
pub const MASK_BITS_B: u64 = P2_BITS_B - 1;
pub const MASK_BITS_C: u64 = P2_BITS_C - 1;

// Circuit parameters
pub const STATE_IN_FIRST_REF: usize = KECCAK_GATE_CONFIG.sin_first_ref as usize;
pub const STATE_IN_GROUP_BY: usize = KECCAK_GATE_CONFIG.sin_ref_group_by as usize;
pub const STATE_IN_NUMBER: usize = KECCAK_GATE_CONFIG.sin_ref_number as usize;
pub const STATE_IN_REF_DISTANCE: usize = KECCAK_GATE_CONFIG.sin_ref_distance as usize;
pub const XOR_GATE_OP: u8 = 0x00;
pub const XOR_ANDP_GATE_OP: u8 = 0x01;
