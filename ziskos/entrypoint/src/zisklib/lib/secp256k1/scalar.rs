//! Operations in the scalar field Fn of the secp256k1 curve

use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::lt,
};

use super::constants::{N, N_MINUS_ONE};

/// Reduces a 256-bit value modulo the secp256k1 curve order N
pub fn reduce_fn_secp256k1(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if lt(x, &N) {
        return *x;
    }

    // x·1 + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Negation in the scalar field of the secp256k1 curve
pub fn neg_fn_secp256k1(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &N_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Addition in the scalar field of the secp256k1 curve
pub fn add_fn_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Multiplication in the scalar field of the secp256k1 curve
pub fn mul_fn_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Subtraction in the scalar field of the secp256k1 curve
pub fn sub_fn_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &N_MINUS_ONE, c: x, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}
