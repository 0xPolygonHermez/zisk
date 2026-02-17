use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;

// Memory layout
pub const PARAMS: usize = 3;
pub const READ_PARAMS: usize = 2;
pub const DIRECT_READ_PARAMS: usize = 1;
pub const DIRECT_READ_PARAM_POS: usize = 0;
pub const WRITE_PARAMS: usize = 1;
pub const RESULT_PARAMS: usize = 0;
pub const PARAM_CHUNKS: usize = 16;
pub const START_READ_PARAMS: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE + PARAMS;

// Generic Parameters
pub const CLOCKS_PER_G: usize = 3;
pub const G_FUNCTIONS_COLUMN_MIXING: usize = 4;
pub const G_FUNCTIONS_DIAGONAL_MIXING: usize = 4;
pub const CLOCKS_COLUMN_MIXING: usize = CLOCKS_PER_G * G_FUNCTIONS_COLUMN_MIXING;
pub const CLOCKS_DIAGONAL_MIXING: usize = CLOCKS_PER_G * G_FUNCTIONS_DIAGONAL_MIXING;
pub const CLOCKS: usize = CLOCKS_COLUMN_MIXING + CLOCKS_DIAGONAL_MIXING;

/// Message word permutation schedule
pub const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

/// Rotation constants for G function
pub const R1_G: u32 = 32;
pub const R2_G: u32 = 24;
pub const R3_G: u32 = 16;
pub const R4_G: u32 = 63;
