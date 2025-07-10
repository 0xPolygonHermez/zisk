use crate::{
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    exp_power_of_two, fcall_secp256k1_fp_inv, fcall_secp256k1_fp_sqrt,
};

use super::constants::{P, P_MINUS_ONE};

pub fn secp256k1_fp_add(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·1 + y
    let mut params = SyscallArith256ModParams {
        a: &x,
        b: &[1, 0, 0, 0],
        c: &y,
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fp_mul(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params = SyscallArith256ModParams {
        a: &x,
        b: &y,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fp_square(x: &[u64; 4]) -> [u64; 4] {
    // x·x + 0
    let mut params = SyscallArith256ModParams {
        a: &x,
        b: &x,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);

    *params.d
}

pub fn secp256k1_fp_sqrt(x: &[u64; 4], parity: u64) -> ([u64; 4], bool) {
    // Hint the sqrt
    match fcall_secp256k1_fp_sqrt(x, parity) {
        // If there is a square root, check that x_sqrt·x_sqrt = x (p)
        Some(x_sqrt) => {
            let mut params = SyscallArith256ModParams {
                a: &x_sqrt,
                b: &x_sqrt,
                c: &[0, 0, 0, 0],
                module: &P,
                d: &mut [0, 0, 0, 0],
            };
            syscall_arith256_mod(&mut params);
            assert_eq!(*params.d, *x);
            (x_sqrt, true)
        }
        // If there is no square root, check that x is a non-quadratic residue
        None => {
            secp256k1_fp_assert_nqr(x);
            ([0u64; 4], false)
        }
    }
}

/// Given a 256-bit number `x`, uses the Euler's Criterion `x^{(p-1)/2} == -1 (mod p)` to assert it is not a quadratic residue.
/// It assumes that `x` is a field element.
fn secp256k1_fp_assert_nqr(x: &[u64; 4]) {
    // Note: (p-1)/2 = 2^255 - 2^32 + 2^31 - 2^9 + 2^4 + 2^3 - 1

    //                x^(2^255) · x^(2^31) · x^(2^4) · x^(2^3)
    // x^((p-1)/2) = ------------------------------------------
    //                     x^(2^32) · x^(2^9) · x

    // Costs: 253 squarings, 9 multiplications

    // Compute the necessary powers of two
    let exp_3 = exp_power_of_two(x, &P, 3);
    let mut params = SyscallArith256ModParams {
        a: &exp_3,
        b: &exp_3,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    let exp_4 = *params.d;
    let exp_9 = exp_power_of_two(&exp_4, &P, 5);
    let exp_31 = exp_power_of_two(&exp_9, &P, 22);
    params.a = &exp_31;
    params.b = &exp_31;
    syscall_arith256_mod(&mut params);
    let exp_32 = *params.d;
    let exp_255 = exp_power_of_two(&exp_32, &P, 223);

    // --> Compute the numerator
    params.a = &exp_255;
    params.b = &exp_31;
    syscall_arith256_mod(&mut params);
    let _res = *params.d;
    params.a = &_res;
    params.b = &exp_4;
    syscall_arith256_mod(&mut params);
    let _res = *params.d;
    params.a = &_res;
    params.b = &exp_3;
    syscall_arith256_mod(&mut params);
    let num = *params.d;

    // --> Compute the denominator
    params.a = &exp_32;
    params.b = &exp_9;
    syscall_arith256_mod(&mut params);
    let _res = *params.d;
    params.a = &_res;
    params.b = x;
    syscall_arith256_mod(&mut params);
    let den = *params.d;

    // --> Compute the result
    // Hint the inverse of the denominator and check it
    let den_inv = fcall_secp256k1_fp_inv(&den);
    params.a = &den;
    params.b = &den_inv;
    syscall_arith256_mod(&mut params);
    assert_eq!(*params.d, [0x1, 0x0, 0x0, 0x0]);

    // Multiply and check the non-quadratic residue
    params.a = &num;
    params.b = &den_inv;
    syscall_arith256_mod(&mut params);
    assert_eq!(*params.d, P_MINUS_ONE);
}
