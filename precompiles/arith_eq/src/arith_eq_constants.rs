//! This module defines constants for the Arith256 precompile.

/// Generic Parameters
pub const ARITH_EQ_ROWS_BY_OP: usize = 16;
pub const ARITH_EQ_CHUNKS: usize = 16;
pub const ARITH_EQ_CHUNK_BITS: usize = 16;
pub const ARITH_EQ_CHUNK_SIZE: usize = 1 << ARITH_EQ_CHUNK_BITS;
pub const ARITH_EQ_CHUNK_BASE_MAX: usize = ARITH_EQ_CHUNK_SIZE - 1;
