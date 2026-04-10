use crate::syscalls::{
    syscall_add256, syscall_arith256, syscall_arith256_mod, SyscallAdd256Params,
    SyscallArith256ModParams, SyscallArith256Params,
};
use crate::zisklib::fcall_uint256_div;
use crate::zisklib::lib::{
    constants::{MINUS_ONE_256 as MINUS_ONE, ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{is_zero, lt},
};

/// Given 256-bit integers `a,b`, it computes `a / b` and `a % b`.
///
/// Panics if `b == 0`.
pub fn div_rem(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], [u64; 4]) {
    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma

    // Hint the quotient and remainder
    let (quo, rem) = fcall_uint256_div(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check the division lemma: a = q·b + r
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a: &quo, b, c: &rem, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(dl, *a);
    assert_eq!(dh, ZERO);

    // Check that: r < b
    assert!(lt(&rem, b), "Remainder must be less than the divisor");

    (quo, rem)
}

/// Given 256-bit integers `a,b`, it computes the ceiling of `a / b`.
///
/// Panics if `b == 0`.
pub fn div_ceil(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    let (quo, rem) = div_rem(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
    if is_zero(&rem) {
        quo
    } else {
        let mut res = ZERO;
        let mut params = SyscallAdd256Params { a: &quo, b: &ZERO, cin: 1, c: &mut res };
        syscall_add256(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        res
    }
}

/// Given 256-bit integers `a,b`, it computes `a / b`.
///
/// Panics if `b == 0`.
pub fn div(a: &[u64; 4], b: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    div_rem(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given 256-bit integers `a,b`, it computes `a % b`.
///
/// Panics if `b == 0`.
pub fn rem(a: &[u64; 4], b: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    if is_zero(b) {
        panic!("Division by zero");
    }

    let mut d = ZERO;
    let mut params = SyscallArith256ModParams { a, b: &ONE, c: &ZERO, module: b, d: &mut d };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    d
}
