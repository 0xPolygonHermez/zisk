// Generic Parameters
pub const STATE_SIZE_BITS: usize = 256;
pub const INPUT_SIZE_BITS: usize = 512;
pub const INPUT_DATA_SIZE_BITS: usize = STATE_SIZE_BITS + INPUT_SIZE_BITS; // 768
pub const INPUT_DATA_SIZE_U32: usize = INPUT_DATA_SIZE_BITS / 32; // 24
pub const OUTPUT_SIZE_BITS: usize = 256;
pub const RB: usize = 32;
pub const RC: usize = 2;
pub const BITS_IN_PARALLEL: usize = 2;

// Sha256f circuit Parameters
pub const BITS_SHA256F: usize = 7;
pub const CHUNKS_SHA256F: usize = 8;
pub const P2_BITS_SHA256F: u64 = 1 << BITS_SHA256F;
pub const P2_CHUNK_BITS_SHA256F: u64 = 1 << (BITS_SHA256F * CHUNKS_SHA256F);
pub const MASK_BITS_SHA256F: u64 = P2_BITS_SHA256F - 1;
pub const MASK_CHUNK_BITS_SHA256F: u64 = P2_CHUNK_BITS_SHA256F - 1;
pub const NUM_SHA256F_PER_CIRCUIT: usize = BITS_SHA256F * CHUNKS_SHA256F;
pub const RB_SIZE: usize = NUM_SHA256F_PER_CIRCUIT * RB * RC / BITS_IN_PARALLEL;
pub const BLOCK_SIZE: usize = NUM_SHA256F_PER_CIRCUIT * 32 / BITS_IN_PARALLEL;
pub const INPUT_SIZE: usize = NUM_SHA256F_PER_CIRCUIT * INPUT_DATA_SIZE_BITS / BITS_IN_PARALLEL;
pub const OUTPUT_SIZE: usize = NUM_SHA256F_PER_CIRCUIT * OUTPUT_SIZE_BITS / BITS_IN_PARALLEL;
pub const IN_DATA_BLOCKS: usize = INPUT_DATA_SIZE_BITS / (RB * RC);
pub const OUT_BLOCKS: usize = OUTPUT_SIZE_BITS / (RB * RC);

// Sha256f Table Parameters
pub const BITS_SHA256F_TABLE: usize = BITS_SHA256F;
pub const CHUNKS_SHA256F_TABLE: usize = 1;
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
pub const XOR_GATE_OP: u8 = 0x00;
pub const CH_GATE_OP: u8 = 0x01;
pub const MAJ_GATE_OP: u8 = 0x02;
pub const ADD_GATE_OP: u8 = 0x03;
