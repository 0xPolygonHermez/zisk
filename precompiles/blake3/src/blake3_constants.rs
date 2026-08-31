use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;

// Memory layout
pub const PARAMS: usize = 2;
pub const READ_PARAMS: usize = 2;
pub const WRITE_PARAMS: usize = 1;
pub const RESULT_PARAMS: usize = 0;
pub const PARAM_CHUNKS: usize = 8;
pub const START_READ_PARAMS: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE + PARAMS;

// Generic Parameters
pub const CLOCKS_PER_G: usize = 1;
pub const NUM_G_PER_ROUND: usize = 8;
pub const NUM_ROUNDS: usize = 7;
pub const CLOCKS: usize = CLOCKS_PER_G * NUM_G_PER_ROUND * NUM_ROUNDS;

// Blake3f XOR⊕ROTR table: 8-bit A × 8-bit B × rotation ∈ {0, 12}
pub const BLAKE3F_TABLE_SIZE: usize = 1 << 17;

/// Message word permutation schedule (cumulative BLAKE3 permutations, one row per round)
pub const SIGMA: [[usize; 16]; NUM_ROUNDS] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8],
    [3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1],
    [10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6],
    [12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4],
    [9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7],
    [11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13],
];

/// Rotation constants for G function
pub const R1_G: u32 = 16;
pub const R2_G: u32 = 12;
pub const R3_G: u32 = 8;
pub const R4_G: u32 = 7;
