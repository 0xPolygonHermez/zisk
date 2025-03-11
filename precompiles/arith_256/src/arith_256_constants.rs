//! This module defines constants for the Arith256 precompile.

/// Generic Parameters
pub const ARITH_256_ROWS_BY_OP: usize = 16;
pub const ARITH_256_CHUNKS: usize = 16;
pub const ARITH_256_CHUNK_BITS: usize = 16;
pub const ARITH_256_CHUNK_SIZE: usize = 1 << ARITH_256_CHUNK_BITS;
pub const ARITH_256_CHUNK_BASE_MAX: usize = ARITH_256_CHUNK_SIZE - 1;
