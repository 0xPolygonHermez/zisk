use crate::{
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    fcall_secp256k1_fp_inv,
    zisklib::lib::utils::exp_power_of_two,
};

use super::constants::{P, P_MINUS_ONE};

/// Given a 256-bit number `x`, uses the Euler's Criterion `x^{(p-1)/2} == -1 (mod p)` to assert it is not a quadratic residue.
/// It assumes that `x` is a field element.
pub fn secp256k1_fp_assert_nqr(x: &[u64; 4]) {
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
