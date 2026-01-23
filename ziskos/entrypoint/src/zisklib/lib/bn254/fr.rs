use crate::zisklib::lt;

use super::constants::R;

// Checks if a 256-bit integer `x` is in canonical form
#[inline]
pub fn is_canonical_fr_bn254(x: &[u64; 4]) -> bool {
    lt(x, &R)
}

/// Convert big-endian bytes to little-endian u64 limbs for a scalar (32 bytes -> [u64; 4])
pub fn scalar_bytes_be_to_u64_le_bn254(bytes: &[u8; 32]) -> [u64; 4] {
    let mut result = [0u64; 4];

    for i in 0..4 {
        for j in 0..8 {
            result[3 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    result
}
