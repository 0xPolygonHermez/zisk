use crate::{
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    fcall_secp256r1_fn_inv,
};

use super::constants::N;

pub fn secp256r1_fn_mul(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);

    *params.d
}

/// Inverts a non-zero element `x`
pub fn secp256r1_fn_inv(x: &[u64; 4]) -> [u64; 4] {
    // Hint the inverse
    let x_inv = fcall_secp256r1_fn_inv(x);

    // Check that x·x_inv = 1 (N)
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &x_inv,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    assert_eq!(*params.d, [0x1, 0x0, 0x0, 0x0]);

    x_inv
}
