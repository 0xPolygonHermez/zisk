use crate::{
    syscalls::{
        syscall_arith256, syscall_arith256_mod, SyscallArith256ModParams, SyscallArith256Params,
    },
    zisklib::{eq, fcall_bigint256_div, fcall_msb_pos_256, lt},
};

pub fn mul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], [u64; 4]) {
    let mut params =
        SyscallArith256Params { a, b, c: &[0u64; 4], dl: &mut [0u64; 4], dh: &mut [0u64; 4] };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (*params.dl, *params.dh)
}

pub fn wmul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    let mut params =
        SyscallArith256Params { a, b, c: &[0u64; 4], dl: &mut [0u64; 4], dh: &mut [0u64; 4] };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.dl
}

pub fn divrem256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], [u64; 4]) {
    // Check for division by zero
    assert!(!eq(b, &[0u64; 4]), "Division by zero");

    // Hint the result of the division
    let (quotient, remainder) = fcall_bigint256_div(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check that a = b * quotient + remainder and remainder < b
    assert!(lt(&remainder, b), "Remainder is not less than divisor");
    let mut params = SyscallArith256Params {
        a: b,
        b: &quotient,
        c: &remainder,
        dl: &mut [0u64; 4],
        dh: &mut [0u64; 4],
    };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert!(eq(params.dl, a), "Dividend does not equal divisor * quotient + remainder");

    (quotient, remainder)
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
pub fn exp_power_of_two(
    x: &[u64; 4],
    module: &[u64; 4],
    power_log: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x^1 = x
    if power_log == 0 {
        return *x;
    }

    let mut result = *x;
    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params = SyscallArith256ModParams {
            a: &result,
            b: &result,
            c: &zero,
            module,
            d: &mut [0u64; 4],
        };
        syscall_arith256_mod(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        result = *params.d;
    }

    result
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
pub fn exp_power_of_two_self(
    x: &mut [u64; 4],
    module: &[u64; 4],
    power_log: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    if power_log == 0 {
        return;
    }

    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params =
            SyscallArith256ModParams { a: x, b: x, c: &zero, module, d: &mut [0u64; 4] };
        syscall_arith256_mod(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        *x = *params.d;
    }
}

pub fn wpow256(
    a: &[u64; 4],
    exp: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // 0^0 = 1 by convention
    // 0^n = 0 for n > 0
    if eq(a, &[0u64; 4]) {
        return if eq(exp, &[0u64; 4]) { [1, 0, 0, 0] } else { [0u64; 4] };
    }

    // Direct cases: exp = 0,1,2
    match exp {
        [0, 0, 0, 0] => {
            // Return a^0 = 1
            return [1, 0, 0, 0];
        }
        [1, 0, 0, 0] => {
            // Return a
            return *a;
        }
        [2, 0, 0, 0] => {
            // Return a^2
            let mut dl = [0u64; 4];
            let mut dh = [0u64; 4];
            let mut params =
                SyscallArith256Params { a, b: a, c: &[0u64; 4], dl: &mut dl, dh: &mut dh };
            syscall_arith256(
                &mut params,
                #[cfg(feature = "hints")]
                hints,
            );
            return dl;
        }
        _ => {}
    }

    // We can assume exp > 2 from now on
    // Hint the length the binary representations of exp
    // We will verify the output by recomposing exp
    let (max_limb, max_bit) = fcall_msb_pos_256(
        exp,
        &[0, 0, 0, 0],
        #[cfg(feature = "hints")]
        hints,
    );

    // Perform the loop, based on the binary representation of exp

    // We do the first iteration separately
    let _max_limb = max_limb as usize;
    let exp_bit = (exp[_max_limb] >> max_bit) & 1;
    assert_eq!(exp_bit, 1); // the first received bit should be 1

    // Start at a
    let mut result = *a;
    let mut exp_rec = [0, 0, 0, 0];
    exp_rec[_max_limb] = 1 << max_bit;

    // Perform the rest of the loop
    let _max_bit = max_bit as usize;
    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    for i in (0..=_max_limb).rev() {
        let bit_len = if i == _max_limb { _max_bit - 1 } else { 63 };
        for j in (0..=bit_len).rev() {
            // Always square
            let mut params = SyscallArith256Params {
                a: &result,
                b: &result,
                c: &[0u64; 4],
                dl: &mut dl,
                dh: &mut dh,
            };
            syscall_arith256(
                &mut params,
                #[cfg(feature = "hints")]
                hints,
            );
            result = dl;

            // Get the next bit b of exp
            // If b == 1, we multiply result by a, otherwise start the next iteration
            if ((exp[i] >> j) & 1) == 1 {
                let mut params = SyscallArith256Params {
                    a: &result,
                    b: a,
                    c: &[0u64; 4],
                    dl: &mut dl,
                    dh: &mut dh,
                };
                syscall_arith256(
                    &mut params,
                    #[cfg(feature = "hints")]
                    hints,
                );
                result = dl;

                // Reconstruct exp
                exp_rec[i] |= 1 << j;
            }
        }
    }

    // Check that the reconstructed exp is equal to the input exp
    assert_eq!(exp_rec, *exp);

    result
}

/// Modular reduction of a 256-bit integer
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `m` must point to a valid `[u64; 4]` (32 bytes).
/// - `result` must point to a valid `[u64; 4]` (32 bytes), used as output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_redmod256_c")]
pub unsafe extern "C" fn redmod256_c(
    a: *const u64,
    m: *const u64,
    result: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: &*(a as *const [u64; 4]),
        b: &[1, 0, 0, 0],
        c: &[0u64; 4],
        module: &*(m as *const [u64; 4]),
        d: &mut d,
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(d.as_ptr(), result, 4);
}

/// Modular addition of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
/// - `m` must point to a valid `[u64; 4]` (32 bytes).
/// - `result` must point to a valid `[u64; 4]` (32 bytes), used as output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_addmod256_c")]
pub unsafe extern "C" fn addmod256_c(
    a: *const u64,
    b: *const u64,
    m: *const u64,
    result: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: &*(a as *const [u64; 4]),
        b: &[1, 0, 0, 0],
        c: &*(b as *const [u64; 4]),
        module: &*(m as *const [u64; 4]),
        d: &mut d,
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(d.as_ptr(), result, 4);
}

/// Modular multiplication of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
/// - `m` must point to a valid `[u64; 4]` (32 bytes).
/// - `result` must point to a valid `[u64; 4]` (32 bytes), used as output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mulmod256_c")]
pub unsafe extern "C" fn mulmod256_c(
    a: *const u64,
    b: *const u64,
    m: *const u64,
    result: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: &*(a as *const [u64; 4]),
        b: &*(b as *const [u64; 4]),
        c: &[0u64; 4],
        module: &*(m as *const [u64; 4]),
        d: &mut d,
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(d.as_ptr(), result, 4);
}

/// Wrapping multiplication of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
/// - `result` must point to a valid `[u64; 4]` (32 bytes), used as output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wmul256_c")]
pub unsafe extern "C" fn wmul256_c(
    a: *const u64,
    b: *const u64,
    result: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    let mut params = SyscallArith256Params {
        a: &*(a as *const [u64; 4]),
        b: &*(b as *const [u64; 4]),
        c: &[0u64; 4],
        dl: &mut dl,
        dh: &mut dh,
    };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(dl.as_ptr(), result, 4);
}

/// Overflowing multiplication of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
/// - `result` must point to a valid `[u64; 4]` (32 bytes), used as output.
///
/// Returns `true` if overflow occurred, `false` otherwise.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_omul256_c")]
pub unsafe extern "C" fn omul256_c(
    a: *const u64,
    b: *const u64,
    result: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    let mut params = SyscallArith256Params {
        a: &*(a as *const [u64; 4]),
        b: &*(b as *const [u64; 4]),
        c: &[0u64; 4],
        dl: &mut dl,
        dh: &mut dh,
    };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(dl.as_ptr(), result, 4);

    // If the high part is non-zero, we have an overflow
    !eq(&dh, &[0u64; 4])
}

/// Division and remainder of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `b` must point to a valid `[u64; 4]` (32 bytes), and must be non-zero.
/// - `q` must point to a valid `[u64; 4]` (32 bytes), used as quotient output.
/// - `r` must point to a valid `[u64; 4]` (32 bytes), used as remainder output.
///
/// # Panics
/// Panics if `b` is zero.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_divrem256_c")]
pub unsafe extern "C" fn divrem256_c(
    a: *const u64,
    b: *const u64,
    q: *mut u64,
    r: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);
    let b_ref = &*(b as *const [u64; 4]);

    // Check for division by zero
    assert!(!eq(b_ref, &[0u64; 4]), "Division by zero");

    // Hint the result of the division
    let (quotient, remainder) = fcall_bigint256_div(
        a_ref,
        b_ref,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check that a = b * quotient + remainder and remainder < b
    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    let mut params =
        SyscallArith256Params { a: b_ref, b: &quotient, c: &remainder, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert!(eq(&dl, a_ref), "Dividend does not equal divisor * quotient + remainder");
    assert!(lt(&remainder, b_ref), "Remainder is not less than divisor");

    core::ptr::copy_nonoverlapping(quotient.as_ptr(), q, 4);
    core::ptr::copy_nonoverlapping(remainder.as_ptr(), r, 4);
}

/// Wrapping exponentiation of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes).
/// - `exp` must point to a valid `[u64; 4]` (32 bytes).
/// - `result` must point to a valid `[u64; 4]` (32 bytes), used as output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wpow256_c")]
pub unsafe extern "C" fn wpow256_c(
    a: *const u64,
    exp: *const u64,
    result: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);
    let exp_ref = &*(exp as *const [u64; 4]);

    let res = wpow256(
        a_ref,
        exp_ref,
        #[cfg(feature = "hints")]
        hints,
    );
    core::ptr::copy_nonoverlapping(res.as_ptr(), result, 4);
}
