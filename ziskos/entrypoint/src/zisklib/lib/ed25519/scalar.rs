//! Operations in the scalar field of the Ed25519 curve (order L of the base point)

use crate::zisklib::{le_bytes_to_u64_4, lt};

use super::{
    constants::{L, R256_MOD_L},
    field::arith_mod,
};

/// Whether a scalar is canonical, i.e. `s < L` (RFC 8032 rejects signatures with `s >= L`).
pub fn is_canonical_scalar_ed25519(s: &[u64; 4]) -> bool {
    lt(s, &L)
}

/// Reduces a 512-bit little-endian integer (e.g. a SHA-512 digest) mod L.
///
/// With `h = hi·2^256 + lo`, `h ≡ hi·(2^256 mod L) + lo (mod L)`: a single `arith256_mod`.
pub fn reduce_wide_scalar_ed25519(
    bytes: &[u8; 64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    let lo = le_bytes_to_u64_4(bytes[0..32].try_into().unwrap());
    let hi = le_bytes_to_u64_4(bytes[32..64].try_into().unwrap());
    arith_mod(
        &hi,
        &R256_MOD_L,
        &lo,
        &L,
        #[cfg(feature = "hints")]
        hints,
    )
}
