//! This module defines constants for binary operation opcodes in both 32-bit and 64-bit variants.
//!
//! These constants are derived from the `ZiskOp` enum and represent the numeric opcodes for each
//! operation.

/// Keccakf parameters
pub const CHUNKS: u64 = 5;
pub const BITS: u64 = 12;
pub const P2_BITS: u64 = 1 << BITS;
pub const P2_BITS_SQUARED: u64 = P2_BITS * P2_BITS;
pub const MASK_BITS: u64 = P2_BITS - 1;

/// Keccakf gate opcodes
pub const XOR_GATE_OP: u8 = 0x00;
pub const ANDP_GATE_OP: u8 = 0x01;
