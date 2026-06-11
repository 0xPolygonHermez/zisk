use crate::zisklib::lib::{
    constants::{MAX_256 as MAX, ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{is_one, is_zero},
};
use crate::zisklib::{fcall_bin_decomp, fcall_msb_pos_256, is_power_of_two};

use super::mul::{overflowing_mul256, overflowing_square256, wrapping_mul256, wrapping_square256};

/// Given 256-bit integers `base` and `exp`, it computes `base^exp (mod 2^256)`.
/// Returns `None` if overflow occurred.
pub fn checked_pow256(
    base: &[u64; 4],
    exp: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    match overflowing_pow256(
        base,
        exp,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => Some(res),
        (_, true) => None,
    }
}

/// Given 256-bit integers `base` and `exp`, it computes `base^exp (mod 2^256)`.
/// Returns `(result, overflow)` where `result` is the computed power and `overflow`
pub fn overflowing_pow256(
    base: &[u64; 4],
    exp: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    // Early returns
    if is_zero(exp) {
        // base^0 = 1 (includes 0^0)
        return (ONE, false);
    } else if is_one(exp) {
        // base^1 = base
        return (*base, false);
    }

    if is_zero(base) {
        // 0^exp = 0
        return (ZERO, false);
    } else if is_one(base) {
        // 1^exp = 1
        return (ONE, false);
    }
    // We can assume exp,base > 1 from now on

    // Optimized path for power-of-two exponents: only squaring is needed
    if is_power_of_two(exp) {
        // Hint which bit is set in the exponent
        let (limb, bit) = fcall_msb_pos_256(
            exp,
            #[cfg(feature = "hints")]
            hints,
        );

        // Bound before use as index/shift
        assert!(limb < 4 && bit < 64, "msb_pos hint out of range");

        // Check that the hinted bit position matches the original exponent
        let mut check_exp = [0u64; 4];
        check_exp[limb as usize] = 1u64 << (bit as usize);
        assert_eq!(check_exp, *exp, "Exponent bit position mismatch");

        // Perform repeated squaring for the single set bit in the exponent
        let mut overflow = false;
        let mut result = *base;
        for _ in 0..bit {
            let (res, sq_overflow) = overflowing_square256(
                &result,
                #[cfg(feature = "hints")]
                hints,
            );
            result = res;
            overflow |= sq_overflow;
        }

        return (result, overflow);
    }

    // Hint the binary decomposition of the exponent (MSB first)
    let (len, bits) = fcall_bin_decomp(
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

    // The leading bit must be 1 for a non-zero exponent
    assert!(len > 0 && bits[0] == 1, "Exponent must be non-zero");

    // Left-to-right square-and-multiply, starting from the second bit
    let mut overflow = false;
    let mut result = *base;
    let mut rec_exp = [0u64; 4];
    let bit_pos = len - 1;
    rec_exp[bit_pos / 64] = 1u64 << (bit_pos % 64);
    for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
        // Compute result = result² (mod 2^256)
        let (res, sq_overflow) = overflowing_square256(
            &result,
            #[cfg(feature = "hints")]
            hints,
        );
        result = res;
        overflow |= sq_overflow;

        if bit == 1 {
            // Compute result = (result * base) (mod 2^256)
            let (res, mul_overflow) = overflowing_mul256(
                &result,
                base,
                #[cfg(feature = "hints")]
                hints,
            );
            result = res;
            overflow |= mul_overflow;

            // Recompose the exponent
            let bit_pos = len - 1 - bit_idx;
            rec_exp[bit_pos / 64] |= 1u64 << (bit_pos % 64);
        }
    }

    // Verify the hinted decomposition matches the original exponent
    assert_eq!(rec_exp, *exp, "Exponent decomposition mismatch");

    (result, overflow)
}

/// Given 256-bit integers `base` and `exp`, it computes `base^exp (mod 2^256)`.
pub fn wrapping_pow256(
    base: &[u64; 4],
    exp: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // Early returns
    if is_zero(exp) {
        // base^0 = 1 (includes 0^0)
        return ONE;
    } else if is_one(exp) {
        // base^1 = base
        return *base;
    }

    if is_zero(base) {
        // 0^exp = 0
        return ZERO;
    } else if is_one(base) {
        // 1^exp = 1
        return ONE;
    }
    // We can assume exp,base > 1 from now on

    if is_power_of_two(exp) {
        // Hint which bit is set in the exponent
        let (limb, bit) = fcall_msb_pos_256(
            exp,
            #[cfg(feature = "hints")]
            hints,
        );

        // Bound before use as index/shift
        assert!(limb < 4 && bit < 64, "msb_pos hint out of range");

        // Check that the hinted bit position matches the original exponent
        let mut check_exp = [0u64; 4];
        check_exp[limb as usize] = 1u64 << (bit as usize);
        assert_eq!(check_exp, *exp, "Exponent bit position mismatch");

        // Perform repeated squaring for the single set bit in the exponent
        let mut result = *base;
        for _ in 0..bit {
            result = wrapping_square256(
                &result,
                #[cfg(feature = "hints")]
                hints,
            );
        }

        return result;
    }

    // Hint the binary decomposition of the exponent (MSB first)
    let (len, bits) = fcall_bin_decomp(
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

    // The leading bit must be 1 for a non-zero exponent
    assert!(len > 0 && bits[0] == 1, "Exponent must be non-zero");

    // Left-to-right square-and-multiply, starting from the second bit
    let mut result = *base;
    let mut rec_exp = [0u64; 4];
    let bit_pos = len - 1;
    rec_exp[bit_pos / 64] = 1u64 << (bit_pos % 64);
    for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
        // Compute result = result² (mod 2^256)
        result = wrapping_square256(
            &result,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit == 1 {
            // Compute result = (result * base) (mod 2^256)
            result = wrapping_mul256(
                &result,
                base,
                #[cfg(feature = "hints")]
                hints,
            );

            // Recompose the exponent
            let bit_pos = len - 1 - bit_idx;
            rec_exp[bit_pos / 64] |= 1u64 << (bit_pos % 64);
        }
    }

    // Verify the hinted decomposition matches the original exponent
    assert_eq!(rec_exp, *exp, "Exponent decomposition mismatch");

    result
}

/// Given 256-bit integers `base` and `exp`, it computes of `base^exp (mod 2^256)`.
/// Saturates to numeric bounds on overflow.
pub fn saturating_pow256(
    base: &[u64; 4],
    exp: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    match overflowing_pow256(
        base,
        exp,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => res,
        (_, true) => MAX,
    }
}

// ==================== C FFI Functions ====================

/// 256-bit checked exponentiation. Returns 1 if no overflow, 0 if overflow occurred.
///
/// # Safety
/// - `base_ptr` must point to a valid `[u64; 4]` array
/// - `exp_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_pow256_c")]
pub unsafe extern "C" fn checked_pow256_c(
    base_ptr: *const u64,
    exp_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let base = &*(base_ptr as *const [u64; 4]);
    let exp = &*(exp_ptr as *const [u64; 4]);

    match checked_pow256(
        base,
        exp,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(res) => {
            let result = &mut *(result_ptr as *mut [u64; 4]);
            *result = res;
            1
        }
        None => 0,
    }
}

/// 256-bit overflowing exponentiation. Returns 1 if overflow occurred, 0 otherwise.
///
/// # Safety
/// - `base_ptr` must point to a valid `[u64; 4]` array
/// - `exp_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_overflowing_pow256_c")]
pub unsafe extern "C" fn overflowing_pow256_c(
    base_ptr: *const u64,
    exp_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let base = &*(base_ptr as *const [u64; 4]);
    let exp = &*(exp_ptr as *const [u64; 4]);

    let (res, overflow) = overflowing_pow256(
        base,
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;

    overflow as u8
}

/// 256-bit wrapping exponentiation.
///
/// # Safety
/// - `base_ptr` must point to a valid `[u64; 4]` array
/// - `exp_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_pow256_c")]
pub unsafe extern "C" fn wrapping_pow256_c(
    base_ptr: *const u64,
    exp_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let base = &*(base_ptr as *const [u64; 4]);
    let exp = &*(exp_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = wrapping_pow256(
        base,
        exp,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit saturating exponentiation.
///
/// # Safety
/// - `base_ptr` must point to a valid `[u64; 4]` array
/// - `exp_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_saturating_pow256_c")]
pub unsafe extern "C" fn saturating_pow256_c(
    base_ptr: *const u64,
    exp_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let base = &*(base_ptr as *const [u64; 4]);
    let exp = &*(exp_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = saturating_pow256(
        base,
        exp,
        #[cfg(feature = "hints")]
        hints,
    );
}
