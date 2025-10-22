use crate::{
    add256::{syscall_add256, SyscallAdd256Params},
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    fcall_secp256k1_fn_inv, lt,
};

use super::constants::{N, N_COMP, N_MINUS_ONE};

pub fn secp256k1_fn_reduce(x: &[u64; 4]) -> [u64; 4] {
    if lt(x, &N) {
        return *x;
    }

    // Since 2·n > 2^256, computing x (mod n) = x - n = x + (2^256 - n) (mod 2^256)
    let mut params = SyscallAdd256Params { a: x, b: &N_COMP, cin: 0, c: &mut [0, 0, 0, 0] };
    syscall_add256(&mut params);

    *params.c
}

pub fn secp256k1_fn_add(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fn_neg(x: &[u64; 4]) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &N_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fn_sub(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &N_MINUS_ONE, c: x, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fn_mul(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);

    *params.d
}

/// Inverts a non-zero element `x`
pub fn secp256k1_fn_inv(x: &[u64; 4]) -> [u64; 4] {
    // Hint the inverse
    let x_inv = fcall_secp256k1_fn_inv(x);

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
