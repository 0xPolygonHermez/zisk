use crate::zisklib::fcall_bin_decomp;
use crate::zisklib::lib::{
    constants::{ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{is_one, is_zero},
};

use super::mul::{mul256, square256};

/// Given 256-bit integers `base` and `exp`, it computes `base^exp (mod 2^256)`.
pub fn pow(
    base: &[u64; 4],
    exp: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
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
    let mut result = *base;
    let mut rec_exp = [0u64; 4];
    let bit_pos = len - 1;
    rec_exp[bit_pos / 64] = 1u64 << (bit_pos % 64);
    for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
        // Compute result = result² (mod 2^256)
        result = square256(
            &result,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit == 1 {
            // Compute result = (result * base) (mod 2^256)
            result = mul256(
                &result,
                base,
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
