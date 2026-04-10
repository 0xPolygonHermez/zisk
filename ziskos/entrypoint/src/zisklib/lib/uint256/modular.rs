use crate::syscalls::{syscall_arith256_mod, SyscallArith256ModParams};
use crate::zisklib::fcall_bin_decomp;
use crate::zisklib::fcall_uint256_inv_mod;
use crate::zisklib::lib::{
    constants::{ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{gt, is_one, is_zero},
};

/// Given 256-bit integers `a` and `modulus`, it computes `a (mod modulus)`.
pub fn reduce_mod(
    a: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    if gt(a, modulus) {
        let mut d = ZERO;
        let mut params =
            SyscallArith256ModParams { a, b: &ONE, c: &ZERO, module: modulus, d: &mut d };
        syscall_arith256_mod(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        d
    } else {
        *a
    }
}

/// Given 256-bit integers `a,b` and `modulus`, it computes `(a + b) (mod modulus)`.
pub fn add_mod(
    a: &[u64; 4],
    b: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    let mut d = ZERO;
    let mut params = SyscallArith256ModParams { a, b: &ONE, c: b, module: modulus, d: &mut d };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    d
}

/// Given 256-bit integers `a,b` and `modulus`, it computes `(a * b) (mod modulus)`.
pub fn mul_mod(
    a: &[u64; 4],
    b: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    let mut d = ZERO;
    let mut params = SyscallArith256ModParams { a, b, c: &ZERO, module: modulus, d: &mut d };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    d
}

/// Given 256-bit integers `a` and `modulus`, it computes `a^2 (mod modulus)`.
pub fn square_mod(
    a: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    mul_mod(
        a,
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Given 256-bit integers `base`, `exp` and `modulus`, it computes `base^exp (mod modulus)`.
pub fn pow_mod(
    base: &[u64; 4],
    exp: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(modulus) {
        return ZERO;
    }

    // Early returns
    if is_zero(exp) {
        // base^0 = 1 (includes 0^0)
        return ONE;
    } else if is_one(exp) {
        // base^1 = base
        return *base;
    }

    if is_zero(base) {
        // 0^exp = 0
        return ZERO;
    } else if is_one(base) {
        // 1^exp = 1
        return ONE;
    }
    // We can assume exp,base > 1 from now on

    // Hint the binary decomposition of the exponent (MSB first)
    let (len, bits) = fcall_bin_decomp(
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

    // The leading bit must be 1 for a non-zero exponent
    assert!(len > 0 && bits[len - 1] == 1, "Exponent must be non-zero");

    // Left-to-right square-and-multiply, starting from the second bit
    let mut result = reduce_mod(
        base,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rec_exp = [0u64; 4];
    let bit_pos = len - 1;
    rec_exp[bit_pos / 64] = 1u64 << (bit_pos % 64);
    for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
        if is_zero(&result) {
            return ZERO;
        }

        // Compute result = result² (mod modulus)
        result = square_mod(
            &result,
            modulus,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit == 1 {
            // Compute result = (result * base) (mod modulus)
            result = mul_mod(
                &result,
                base,
                modulus,
                #[cfg(feature = "hints")]
                hints,
            );

            // Recompose the exponent
            let bit_pos = len - 1 - bit_idx;
            rec_exp[bit_pos / 64] |= 1u64 << (bit_pos % 64);
        }
    }

    // Verify the hinted decomposition matches the original exponent
    assert_eq!(rec_exp, *exp, "Exponent decomposition mismatch");

    result
}

pub fn inv_mod(
    a: &[u64; 4],
    modulus: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    // Hint the inverse
    let inv = fcall_uint256_inv_mod(
        a,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    if let Some(inv) = inv {
        // Verify: a * inv ≡ 1 (mod modulus)
        let result = mul_mod(
            a,
            &inv,
            modulus,
            #[cfg(feature = "hints")]
            hints,
        );
        assert_eq!(result, ONE, "a * inv must equal 1 mod modulus");

        Some(inv)
    } else {
        None
    }
}
