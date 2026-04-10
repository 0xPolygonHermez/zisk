use crate::syscalls::{syscall_add256, SyscallAdd256Params};
use crate::zisklib::lib::constants::{
    MINUS_ONE_256 as MINUS_ONE, ONE_256 as ONE, ZERO_256 as ZERO,
};

/// Given 256-bit integers `a,b`, it computes `a + b (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates overflow.
pub fn add256_carry(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    let mut c = ZERO;
    let mut params = SyscallAdd256Params { a, b, cin: 0, c: &mut c };
    let cout = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (c, cout != 0)
}

/// Given 256-bit integers `a,b`, it computes `a - b (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates underflow (a < b).
pub fn sub256_carry(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    // a - b = a + (~b + 1); cout=0 means borrow (a < b)
    let not_b = [!b[0], !b[1], !b[2], !b[3]];
    let mut c = ZERO;
    let mut params = SyscallAdd256Params { a, b: &not_b, cin: 1, c: &mut c };
    let cout = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (c, cout == 0)
}

/// Given 256-bit integers `a,b`, it computes `a + b (mod 2^256)`.
pub fn add256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    add256_carry(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given 256-bit integers `a,b`, it computes `a - b (mod 2^256)`.
pub fn sub256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    sub256_carry(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given a 256-bit integer `a`, it computes `-a (mod 2^256)`.
pub fn neg256(a: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    sub256_carry(
        &ZERO,
        a,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}
