use crate::{
    syscalls::{
        syscall_arith256, syscall_arith256_mod, SyscallArith256ModParams, SyscallArith256Params,
    },
    zisklib::{eq, fcall_bigint256_div, fcall_msb_pos_256, lt},
};

pub fn mul256(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    let mut params =
        SyscallArith256Params { a, b, c: &[0u64; 4], dl: &mut [0u64; 4], dh: &mut [0u64; 4] };
    syscall_arith256(&mut params);
    (*params.dl, *params.dh)
}

pub fn wmul256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut params =
        SyscallArith256Params { a, b, c: &[0u64; 4], dl: &mut [0u64; 4], dh: &mut [0u64; 4] };
    syscall_arith256(&mut params);
    *params.dl
}

pub fn divrem256(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    // Hint the result of the division
    let (quotient, remainder) = fcall_bigint256_div(a, b);

    // Check that a = b * quotient + remainder and remainder < b
    assert!(lt(&remainder, b), "Remainder is not less than divisor");
    let mut params = SyscallArith256Params {
        a: b,
        b: &quotient,
        c: &remainder,
        dl: &mut [0u64; 4],
        dh: &mut [0u64; 4],
    };
    syscall_arith256(&mut params);
    assert!(eq(params.dl, a), "Dividend does not equal divisor * quotient + remainder");

    (quotient, remainder)
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
pub fn exp_power_of_two(x: &[u64; 4], module: &[u64; 4], power_log: usize) -> [u64; 4] {
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
        syscall_arith256_mod(&mut params);
        result = *params.d;
    }

    result
}

/// Raises `x` to (2^power_log) modulo `module` using repeated squaring
pub fn exp_power_of_two_self(x: &mut [u64; 4], module: &[u64; 4], power_log: usize) {
    if power_log == 0 {
        return;
    }

    let zero = [0u64; 4];
    for _ in 0..power_log {
        let mut params =
            SyscallArith256ModParams { a: x, b: x, c: &zero, module, d: &mut [0u64; 4] };
        syscall_arith256_mod(&mut params);
        *x = *params.d;
    }
}

pub fn wpow256(a: &[u64; 4], exp: &[u64; 4]) -> [u64; 4] {
    // If a = 0, return 0^0 = 0
    if eq(a, &[0u64; 4]) {
        return [0u64; 4];
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
            syscall_arith256(&mut params);
            return dl;
        }
        _ => {}
    }

    // We can assume exp > 2 from now on
    // Hint the length the binary representations of exp
    // We will verify the output by recomposing exp
    let (max_limb, max_bit) = fcall_msb_pos_256(exp, &[0, 0, 0, 0]);

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
            syscall_arith256(&mut params);
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
                syscall_arith256(&mut params);
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

// ========== Pointer-based API ==========

/// Modular reduction of a 256-bit integer using pointers
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn redmod256_ptr(a: *const u64, m: *const u64, result: *mut u64) {
    debug_assert!(!a.is_null() && !m.is_null() && !result.is_null());

    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: &*(a as *const [u64; 4]),
        b: &[1, 0, 0, 0],
        c: &[0u64; 4],
        module: &*(m as *const [u64; 4]),
        d: &mut d,
    };
    syscall_arith256_mod(&mut params);

    std::ptr::copy_nonoverlapping(d.as_ptr(), result, 4);
}

/// Modular addition of a 256-bit integer using pointers
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `b` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn addmod256_ptr(a: *const u64, b: *const u64, m: *const u64, result: *mut u64) {
    debug_assert!(!a.is_null() && !b.is_null() && !result.is_null());

    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: &*(a as *const [u64; 4]),
        b: &[1, 0, 0, 0],
        c: &*(b as *const [u64; 4]),
        module: &*(m as *const [u64; 4]),
        d: &mut d,
    };
    syscall_arith256_mod(&mut params);

    std::ptr::copy_nonoverlapping(d.as_ptr(), result, 4);
}

/// Modular multiplication of a 256-bit integer using pointers
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `b` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn mulmod256_ptr(a: *const u64, b: *const u64, m: *const u64, result: *mut u64) {
    debug_assert!(!a.is_null() && !b.is_null() && !result.is_null());

    let mut d = [0u64; 4];
    let mut params = SyscallArith256ModParams {
        a: &*(a as *const [u64; 4]),
        b: &*(b as *const [u64; 4]),
        c: &[0u64; 4],
        module: &*(m as *const [u64; 4]),
        d: &mut d,
    };
    syscall_arith256_mod(&mut params);

    std::ptr::copy_nonoverlapping(d.as_ptr(), result, 4);
}

/// Wrapping multiplication of 256-bit integers using pointers
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `b` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn wmul256_ptr(a: *const u64, b: *const u64, result: *mut u64) {
    debug_assert!(!a.is_null() && !b.is_null() && !result.is_null());

    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    let mut params = SyscallArith256Params {
        a: &*(a as *const [u64; 4]),
        b: &*(b as *const [u64; 4]),
        c: &[0u64; 4],
        dl: &mut dl,
        dh: &mut dh,
    };
    syscall_arith256(&mut params);

    std::ptr::copy_nonoverlapping(dl.as_ptr(), result, 4);
}

/// Overflowing multiplication of 256-bit integers using pointers
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `b` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn omul256_ptr(a: *const u64, b: *const u64, result: *mut u64) -> bool {
    debug_assert!(!a.is_null() && !b.is_null() && !result.is_null());

    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    let mut params = SyscallArith256Params {
        a: &*(a as *const [u64; 4]),
        b: &*(b as *const [u64; 4]),
        c: &[0u64; 4],
        dl: &mut dl,
        dh: &mut dh,
    };
    syscall_arith256(&mut params);

    std::ptr::copy_nonoverlapping(dl.as_ptr(), result, 4);

    // If the high part is non-zero, we have an overflow
    !eq(&dh, &[0u64; 4])
}

/// Pointer version of divrem256 that works directly with mutable pointers to u64
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `b` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn divrem256_ptr(a: *const u64, b: *const u64, q: *mut u64, r: *mut u64) {
    debug_assert!(!a.is_null() && !b.is_null() && !q.is_null() && !r.is_null());

    // Hint the result of the division
    let (quotient, remainder) =
        fcall_bigint256_div(&*(a as *const [u64; 4]), &*(b as *const [u64; 4]));

    // Check that a = b * quotient + remainder and remainder < b
    let mut dl = [0u64; 4];
    let mut dh = [0u64; 4];
    let mut params = SyscallArith256Params {
        a: &*(b as *const [u64; 4]),
        b: &quotient,
        c: &remainder,
        dl: &mut dl,
        dh: &mut dh,
    };
    syscall_arith256(&mut params);
    assert!(eq(&dl, &*(a as *const [u64; 4])));
    assert!(lt(&remainder, &*(b as *const [u64; 4])));

    std::ptr::copy_nonoverlapping(quotient.as_ptr(), q, 4);
    std::ptr::copy_nonoverlapping(remainder.as_ptr(), r, 4);
}

/// Pointer version of wpow256 that works directly with mutable pointers to u64
///
/// # Safety
///
/// The caller must ensure:
/// - `a` points to a valid array of at least 4 u64 elements
/// - `exp` points to a valid array of at least 4 u64 elements
/// - `result` points to a valid mutable array of at least 4 u64 elements
/// - All pointers are properly aligned
#[inline(always)]
pub unsafe fn wpow256_ptr(a: *const u64, exp: *const u64, result: *mut u64) {
    debug_assert!(!a.is_null() && !exp.is_null() && !result.is_null());

    let a_array = &*(a as *const [u64; 4]);
    let exp_array = &*(exp as *const [u64; 4]);

    let res = wpow256(a_array, exp_array);

    std::ptr::copy_nonoverlapping(res.as_ptr(), result, 4);
}
