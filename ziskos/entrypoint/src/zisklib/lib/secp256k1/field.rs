use crate::{
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    fcall_secp256k1_fp_inv, fcall_secp256k1_fp_sqrt, lt,
};

use super::constants::{NQR, P, P_MINUS_ONE};

pub fn secp256k1_fp_reduce(x: &[u64; 4]) -> [u64; 4] {
    if lt(x, &P) {
        return *x;
    }

    // x·1 + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    *params.d
}

pub fn secp256k1_fp_add(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

pub fn secp256k1_fp_negate(x: &[u64; 4]) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fp_mul(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fp_mul_scalar(x: &[u64; 4], scalar: u64) -> [u64; 4] {
    // x·scalar + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[scalar, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fp_square(x: &[u64; 4]) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);

    *params.d
}

/// Inverts a non-zero element `x`
pub fn secp256k1_fp_inv(x: &[u64; 4]) -> [u64; 4] {
    // Hint the inverse
    let x_inv = fcall_secp256k1_fp_inv(x);

    // Check that x·x_inv = 1 (P)
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &x_inv,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    assert_eq!(*params.d, [0x1, 0x0, 0x0, 0x0]);

    x_inv
}

pub fn secp256k1_fp_sqrt(x: &[u64; 4], parity: u64) -> ([u64; 4], bool) {
    // Hint the sqrt
    let hint = fcall_secp256k1_fp_sqrt(x, parity);
    let is_qr = hint[0] == 1;
    let sqrt = hint[1..5].try_into().unwrap();

    // Compute sqrt * sqrt
    let mut params = SyscallArith256ModParams {
        a: &sqrt,
        b: &sqrt,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    if is_qr {
        // Check that sqrt * sqrt == x
        assert_eq!(*params.d, *x);
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = secp256k1_fp_mul(x, &NQR);
        assert_eq!(*params.d, nqr);
        (sqrt, false)
    }
}
