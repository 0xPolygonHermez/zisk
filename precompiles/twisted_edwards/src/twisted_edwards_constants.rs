#![allow(dead_code)]
//! Constants shared by every curve of the twisted-Edwards point-add precompile.

/// Generic chunk parameters (shared with the AIR-equation generator).
pub const TWISTED_EDWARDS_ROWS_BY_OP: usize = 16;
pub const TWISTED_EDWARDS_CHUNKS: usize = 16;
pub const TWISTED_EDWARDS_CHUNK_BITS: usize = 16;
pub const TWISTED_EDWARDS_CHUNK_SIZE: usize = 1 << TWISTED_EDWARDS_CHUNK_BITS;
pub const TWISTED_EDWARDS_CHUNK_BASE_MAX: usize = TWISTED_EDWARDS_CHUNK_SIZE - 1;

/// One add operation per curve. The `sel_op` slots follow the curve-flag order of the
/// `TwistedEdwards` airtemplate when every curve is instantiated (`curves: TE_ALL`).
pub const TWISTED_EDWARDS_OP_NUM: usize = 2;
pub const SEL_OP_BABYJUBJUB_ADD: usize = 0;
pub const SEL_OP_ED25519_ADD: usize = 1;
