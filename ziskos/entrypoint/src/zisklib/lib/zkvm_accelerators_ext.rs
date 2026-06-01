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

use zkvm_interface::{
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
