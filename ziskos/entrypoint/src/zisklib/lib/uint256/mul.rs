use crate::syscalls::{syscall_arith256, SyscallArith256Params};
use crate::zisklib::fcall_uint256_inv;
use crate::zisklib::lib::constants::{
    MINUS_ONE_256 as MINUS_ONE, ONE_256 as ONE, ZERO_256 as ZERO,
};

/// Given 256-bit integers `a,b`, it computes `a * b (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates overflow (high 256 bits are non-zero).
pub fn mul256_carry(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a, b, c: &ZERO, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (dl, dh != ZERO)
}

/// Given a 256-bit integer `a`, it computes `a^2 (mod 2^256)`.
pub fn square256(a: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    mul256_carry(
        a,
        a,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given 256-bit integers `a,b`, it computes `a * b (mod 2^256)`.
pub fn mul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    mul256_carry(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given a 256-bit integer `a`, it computes `a^(-1) (mod 2^256)`, if it exists.
/// Returns `None` if `a` is not invertible.
pub fn inv256(a: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> Option<[u64; 4]> {
    // Hint the inverse
    let inv = fcall_uint256_inv(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    if let Some(inv) = inv {
        // Verify: a * inv ≡ 1 (mod 2^256)
        let result = mul256(
            a,
            &inv,
            #[cfg(feature = "hints")]
            hints,
        );
        assert_eq!(result, ONE, "a * inv must equal 1 mod 2^256");

        Some(inv)
    } else {
        None
    }
}
