use crate::syscalls::{syscall_arith256_mod, SyscallArith256ModParams};

use super::constants::P;

pub fn secp256r1_fp_add(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

pub fn secp256r1_fp_mul(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

pub fn secp256r1_fp_square(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}
