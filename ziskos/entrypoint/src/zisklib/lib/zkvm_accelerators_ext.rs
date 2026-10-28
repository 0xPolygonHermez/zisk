//! Non-standard ZisK accelerator extensions.
//!
//! These are ZisK-specific 256-bit modular-arithmetic accelerators that are NOT part of the
//! standard EVM-precompile C interface in `zkvm_accelerators.h` (implemented in
//! `zkvm_accelerators.rs`). They are declared separately in `zkvm_accelerators_ext.h` and may be
//! promoted into the standard interface if/when standardized.
//!
//! All operands are 32-byte big-endian. Each wrapper records the underlying `arith256_mod` /
//! `fcall` operations as hints so the host hint processor can reproduce the witness.
#![allow(clippy::missing_safety_doc)]

use zisk_zkvm_interface::{
    zkvm_status, zkvm_status_ZKVM_EFAIL as ZKVM_EFAIL, zkvm_status_ZKVM_EOK as ZKVM_EOK,
};

/// 256-bit modular multiplication: output = (a * b) mod m
///
/// Used by the EVM MULMOD opcode (0x09). All operands are 32-byte big-endian.
///
/// @param a Pointer to first operand (32 bytes, big-endian)
/// @param b Pointer to second operand (32 bytes, big-endian)
/// @param m Pointer to modulus (32 bytes, big-endian)
/// @param[out] output Pointer to output buffer (32 bytes, big-endian)
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_mulmod256")]
pub unsafe extern "C" fn zkvm_mulmod256(
    a: *const u8,
    b: *const u8,
    m: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        super::mul_mod_bytes256_c(a, b, m, output, hints);
        ZKVM_EOK
    }

    #[cfg(not(feature = "hints"))]
    {
        #[cfg(zisk_hints)]
        unsafe {
            crate::hints::hint_mulmod256(a, b, m);
        }

        #[cfg(zisk_hints_debug)]
        crate::hint_log("hint_mulmod256".to_string());

        super::mul_mod_bytes256_c(a, b, m, output);
        ZKVM_EOK
    }
}

/// 256-bit modular reduction: output = a mod m
///
/// ZisK extension (not a standard EVM precompile). All operands are 32-byte big-endian.
///
/// @param a Pointer to operand (32 bytes, big-endian)
/// @param m Pointer to modulus (32 bytes, big-endian)
/// @param[out] output Pointer to output buffer (32 bytes, big-endian)
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_reduce_mod256")]
pub unsafe extern "C" fn zkvm_reduce_mod256(
    a: *const u8,
    m: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        super::reduce_mod_bytes256_c(a, m, output, hints);
        ZKVM_EOK
    }
    #[cfg(not(feature = "hints"))]
    {
        #[cfg(zisk_hints)]
        unsafe {
            crate::hints::hint_reduce_mod256(a, m);
        }

        #[cfg(zisk_hints_debug)]
        crate::hint_log("hint_reduce_mod256".to_string());

        super::reduce_mod_bytes256_c(a, m, output);
        ZKVM_EOK
    }
}

/// 256-bit modular addition: output = (a + b) mod m
///
/// ZisK extension (not a standard EVM precompile). All operands are 32-byte big-endian.
///
/// @param a Pointer to first operand (32 bytes, big-endian)
/// @param b Pointer to second operand (32 bytes, big-endian)
/// @param m Pointer to modulus (32 bytes, big-endian)
/// @param[out] output Pointer to output buffer (32 bytes, big-endian)
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_add_mod256")]
pub unsafe extern "C" fn zkvm_add_mod256(
    a: *const u8,
    b: *const u8,
    m: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        super::add_mod_bytes256_c(a, b, m, output, hints);
        ZKVM_EOK
    }
    #[cfg(not(feature = "hints"))]
    {
        #[cfg(zisk_hints)]
        unsafe {
            crate::hints::hint_add_mod256(a, b, m);
        }

        #[cfg(zisk_hints_debug)]
        crate::hint_log("hint_add_mod256".to_string());

        super::add_mod_bytes256_c(a, b, m, output);
        ZKVM_EOK
    }
}

/// 256-bit modular squaring: output = a² mod m
///
/// ZisK extension (not a standard EVM precompile). All operands are 32-byte big-endian.
///
/// @param a Pointer to operand (32 bytes, big-endian)
/// @param m Pointer to modulus (32 bytes, big-endian)
/// @param[out] output Pointer to output buffer (32 bytes, big-endian)
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_square_mod256")]
pub unsafe extern "C" fn zkvm_square_mod256(
    a: *const u8,
    m: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        super::square_mod_bytes256_c(a, m, output, hints);
        ZKVM_EOK
    }
    #[cfg(not(feature = "hints"))]
    {
        #[cfg(zisk_hints)]
        unsafe {
            crate::hints::hint_square_mod256(a, m);
        }

        #[cfg(zisk_hints_debug)]
        crate::hint_log("hint_square_mod256".to_string());

        super::square_mod_bytes256_c(a, m, output);
        ZKVM_EOK
    }
}

/// 256-bit modular exponentiation: output = base^exp mod m
///
/// ZisK extension (not a standard EVM precompile). All operands are 32-byte big-endian.
///
/// @param base Pointer to base (32 bytes, big-endian)
/// @param exp Pointer to exponent (32 bytes, big-endian)
/// @param m Pointer to modulus (32 bytes, big-endian)
/// @param[out] output Pointer to output buffer (32 bytes, big-endian)
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_pow_mod256")]
pub unsafe extern "C" fn zkvm_pow_mod256(
    base: *const u8,
    exp: *const u8,
    m: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        super::pow_mod_bytes256_c(base, exp, m, output, hints);
        ZKVM_EOK
    }
    #[cfg(not(feature = "hints"))]
    {
        #[cfg(zisk_hints)]
        unsafe {
            crate::hints::hint_pow_mod256(base, exp, m);
        }

        #[cfg(zisk_hints_debug)]
        crate::hint_log("hint_pow_mod256".to_string());

        super::pow_mod_bytes256_c(base, exp, m, output);
        ZKVM_EOK
    }
}

/// 256-bit modular inverse: output = a⁻¹ mod m
///
/// ZisK extension (not a standard EVM precompile). All operands are 32-byte big-endian.
///
/// @param a Pointer to operand (32 bytes, big-endian)
/// @param m Pointer to modulus (32 bytes, big-endian)
/// @param[out] output Pointer to output buffer (32 bytes, big-endian)
/// @return ZKVM_EOK if the inverse exists, ZKVM_EFAIL otherwise
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_inv_mod256")]
pub unsafe extern "C" fn zkvm_inv_mod256(
    a: *const u8,
    m: *const u8,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        if super::inv_mod_bytes256_c(a, m, output, hints) == 1 {
            ZKVM_EOK
        } else {
            ZKVM_EFAIL
        }
    }
    #[cfg(not(feature = "hints"))]
    {
        #[cfg(zisk_hints)]
        unsafe {
            crate::hints::hint_inv_mod256(a, m);
        }

        #[cfg(zisk_hints_debug)]
        crate::hint_log("hint_inv_mod256".to_string());

        if super::inv_mod_bytes256_c(a, m, output) == 1 {
            ZKVM_EOK
        } else {
            ZKVM_EFAIL
        }
    }
}

/// Shared body of [`zkvm_ed25519_verify`] and [`zkvm_ed25519_verify_strict`].
#[inline(always)]
unsafe fn ed25519_verify(
    pk: *const u8,
    msg: *const u8,
    msg_len: usize,
    sig: *const u8,
    strict: bool,
    verified: *mut bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    // An empty message may come as a null pointer: never build a slice from it.
    let msg = if msg_len == 0 { core::ptr::NonNull::<u8>::dangling().as_ptr() } else { msg };

    #[cfg(feature = "hints")]
    {
        *verified = super::ed25519_verify_c(pk, msg, msg_len, sig, strict, hints);
        ZKVM_EOK
    }
    #[cfg(not(feature = "hints"))]
    {
        *verified = super::ed25519_verify_c(pk, msg, msg_len, sig, strict);
        ZKVM_EOK
    }
}

/// Ed25519 signature verification (RFC 8032, pure Ed25519), with `ed25519-dalek` `verify`
/// semantics: cofactorless, `S >= L` rejected, `A` decompressed as `curve25519-dalek` does.
///
/// ZisK extension (not a standard EVM precompile). RFC 8032 encodings.
///
/// @param pk Pointer to the compressed public key A (32 bytes)
/// @param msg Pointer to the message (`msg_len` bytes; may be NULL if `msg_len` is 0)
/// @param msg_len Message length in bytes
/// @param sig Pointer to the signature R || S (64 bytes)
/// @param[out] verified true if the signature is valid
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_ed25519_verify")]
pub unsafe extern "C" fn zkvm_ed25519_verify(
    pk: *const u8,
    msg: *const u8,
    msg_len: usize,
    sig: *const u8,
    verified: *mut bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    ed25519_verify(
        pk,
        msg,
        msg_len,
        sig,
        false,
        verified,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Ed25519 signature verification with `ed25519-dalek` `verify_strict` semantics: as
/// [`zkvm_ed25519_verify`], and additionally `R` must decompress and neither `A` nor `R` may
/// have small order.
///
/// ZisK extension (not a standard EVM precompile). RFC 8032 encodings.
///
/// @param pk Pointer to the compressed public key A (32 bytes)
/// @param msg Pointer to the message (`msg_len` bytes; may be NULL if `msg_len` is 0)
/// @param msg_len Message length in bytes
/// @param sig Pointer to the signature R || S (64 bytes)
/// @param[out] verified true if the signature is valid
/// @return ZKVM_EOK on success
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_ed25519_verify_strict")]
pub unsafe extern "C" fn zkvm_ed25519_verify_strict(
    pk: *const u8,
    msg: *const u8,
    msg_len: usize,
    sig: *const u8,
    verified: *mut bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    ed25519_verify(
        pk,
        msg,
        msg_len,
        sig,
        true,
        verified,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Ed25519 point decompression with `curve25519-dalek` `CompressedEdwardsY::decompress`
/// semantics (`y` taken mod p, a sign bit on `x = 0` ignored).
///
/// ZisK extension (not a standard EVM precompile).
///
/// @param compressed Pointer to the compressed point (32 bytes, RFC 8032 encoding)
/// @param[out] point Pointer to output buffer (64 bytes): x || y, each 32-byte little-endian
///             and canonical
/// @return ZKVM_EOK if `compressed` encodes a curve point, ZKVM_EFAIL otherwise
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_ed25519_decompress")]
pub unsafe extern "C" fn zkvm_ed25519_decompress(
    compressed: *const u8,
    point: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        if super::decompress_ed25519_c(compressed, point, hints) {
            ZKVM_EOK
        } else {
            ZKVM_EFAIL
        }
    }
    #[cfg(not(feature = "hints"))]
    {
        if super::decompress_ed25519_c(compressed, point) {
            ZKVM_EOK
        } else {
            ZKVM_EFAIL
        }
    }
}

/// Ed25519 double scalar multiplication with the base point: result = [k]A + [s]B, with B the
/// Ed25519 base point. Variable time (Straus), as curve25519-dalek
/// `EdwardsPoint::vartime_double_scalar_mul_basepoint`.
///
/// ZisK extension (not a standard EVM precompile).
///
/// @param k Pointer to the scalar of A (32 bytes, little-endian; any 256-bit value)
/// @param point Pointer to the point A (64 bytes): x || y, each 32-byte little-endian
/// @param s Pointer to the scalar of B (32 bytes, little-endian; any 256-bit value)
/// @param[out] result Pointer to output buffer (64 bytes): x || y, each 32-byte little-endian
///             and canonical
/// @return ZKVM_EOK on success, ZKVM_EFAIL if A is not a curve point with canonical coordinates
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_zkvm_ed25519_double_scalar_mul_base")]
pub unsafe extern "C" fn zkvm_ed25519_double_scalar_mul_base(
    k: *const u8,
    point: *const u8,
    s: *const u8,
    result: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> zkvm_status {
    #[cfg(feature = "hints")]
    {
        if super::double_scalar_mul_with_base_ed25519_c(k, point, s, result, hints) {
            ZKVM_EOK
        } else {
            ZKVM_EFAIL
        }
    }
    #[cfg(not(feature = "hints"))]
    {
        if super::double_scalar_mul_with_base_ed25519_c(k, point, s, result) {
            ZKVM_EOK
        } else {
            ZKVM_EFAIL
        }
    }
}
