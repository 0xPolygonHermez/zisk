//! Common 256-bit constants in little-endian `[u64; 4]` representation.

/// Zero in 256-bit representation
pub const ZERO_256: [u64; 4] = [0, 0, 0, 0];

/// One in 256-bit representation
pub const ONE_256: [u64; 4] = [1, 0, 0, 0];

/// Two in 256-bit representation
pub const TWO_256: [u64; 4] = [2, 0, 0, 0];

/// Maximum value in 256-bit representation
pub const MAX_256: [u64; 4] =
    [0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF];

/// Minus one in 256-bit representation
pub const MINUS_ONE_256: [u64; 4] = MAX_256;
