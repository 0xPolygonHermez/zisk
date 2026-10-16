//! Ed25519 signature verification (RFC 8032, pure Ed25519 without context or prehash).
//!
//! Two variants:
//! - [`ed25519_verify`]: Cofactorless check `[s]B - [k]A == R`, done by recomputing `R` and
//!   comparing its canonical encoding with the signature bytes; `s >= L` is rejected; `A`
//!   is decompressed as `curve25519-dalek` does.
//! - [`ed25519_verify_strict`]: Additionally requires `R` to decompress and rejects a small-order
//!   `A` or `R` (weak keys and their forgeries).
//!
//! `k = SHA-512(R ‖ A ‖ M) mod L` hashes the encodings exactly as given.

use crate::zisklib::{le_bytes_to_u64_4, Sha512};

use super::{
    curve::{
        compress_ed25519, decompress_ed25519, double_scalar_mul_with_base_ed25519,
        is_small_order_ed25519, neg_point_ed25519,
    },
    scalar::{is_canonical_scalar_ed25519, reduce_wide_scalar_ed25519},
};

/// Verifies an Ed25519 signature.
///
/// - `pk`: 32-byte compressed public key `A`.
/// - `msg`: the message.
/// - `sig`: 64-byte signature `R ‖ S`.
pub fn ed25519_verify(
    pk: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    verify(
        pk,
        msg,
        sig,
        false,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Verifies an Ed25519 signature with checks for small-order `A` or `R`, and an `R` that does not decompress.
pub fn ed25519_verify_strict(
    pk: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    verify(
        pk,
        msg,
        sig,
        true,
        #[cfg(feature = "hints")]
        hints,
    )
}

fn verify(
    pk: &[u8; 32],
    msg: &[u8],
    sig: &[u8; 64],
    strict: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // Split the signature into `R` and `S` components.
    let r_bytes: &[u8; 32] = sig[0..32].try_into().unwrap();
    let s = le_bytes_to_u64_4(sig[32..64].try_into().unwrap());

    // Reject non-canonical `S` early.
    if !is_canonical_scalar_ed25519(&s) {
        return false;
    }

    // Decompress the public key `A`.
    let Some(a) = decompress_ed25519(
        pk,
        #[cfg(feature = "hints")]
        hints,
    ) else {
        return false;
    };

    if strict {
        // Decompress the `R` component.
        let Some(r) = decompress_ed25519(
            r_bytes,
            #[cfg(feature = "hints")]
            hints,
        ) else {
            return false;
        };

        // Reject small-order `R` or `A`.
        if is_small_order_ed25519(
            &r,
            #[cfg(feature = "hints")]
            hints,
        ) || is_small_order_ed25519(
            &a,
            #[cfg(feature = "hints")]
            hints,
        ) {
            return false;
        }
    }

    // k = SHA-512(R ‖ A ‖ M) mod L
    let mut h = Sha512::new();
    h.update(r_bytes);
    h.update(pk);
    h.update(msg);
    let k = reduce_wide_scalar_ed25519(
        &h.finalize(),
        #[cfg(feature = "hints")]
        hints,
    );

    // R' = [k](-A) + [s]B must encode to R
    let expected_r = double_scalar_mul_with_base_ed25519(
        &k,
        &neg_point_ed25519(&a),
        &s,
        #[cfg(feature = "hints")]
        hints,
    );
    compress_ed25519(&expected_r) == *r_bytes
}

/// Pointer-based [`ed25519_verify`] / [`ed25519_verify_strict`] for the C accelerator interface.
///
/// # Safety
/// - `pk` must point to 32 bytes, `sig` to 64 bytes
/// - `msg` must point to `msg_len` bytes (it may be dangling when `msg_len == 0`)
#[inline]
pub(crate) unsafe fn ed25519_verify_c(
    pk: *const u8,
    msg: *const u8,
    msg_len: usize,
    sig: *const u8,
    strict: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let pk: &[u8; 32] = &*(pk as *const [u8; 32]);
    let sig: &[u8; 64] = &*(sig as *const [u8; 64]);
    let msg: &[u8] = if msg_len == 0 { &[] } else { core::slice::from_raw_parts(msg, msg_len) };
    verify(
        pk,
        msg,
        sig,
        strict,
        #[cfg(feature = "hints")]
        hints,
    )
}
