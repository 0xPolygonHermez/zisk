// TODO: It can be speed up by using Montgomery multiplication but knowning that divisions are "free"
// For ref: https://www.microsoft.com/en-us/research/wp-content/uploads/1996/01/j37acmon.pdf

use std::vec;

use super::{rem_long, rem_short, mul_and_reduce, square_and_reduce, U256};

/// Modular exponentiation of three large numbers
///
/// It assumes that modulus > 0 and len(base),len(exp) > 0
pub fn modexp(base: &[U256], exp: &[u64], modulus: &[U256]) -> Vec<U256> {
    let len_b = base.len();
    let len_e = exp.len();
    let len_m = modulus.len();
    #[cfg(debug_assertions)]
    {
        assert_ne!(len_b, 0, "Base must have at least one limb");
        assert_ne!(len_e, 0, "Exponent must have at least one limb");
        assert_ne!(len_m, 0, "Modulus must have at least one limb");

        if len_b > 1 {
            assert_ne!(base.last().unwrap(), &U256::ZERO, "Base must not have leading zeros");
        }
        if len_e > 1 {
            assert_ne!(exp.last().unwrap(), &0, "Exponent must not have leading zeros");
        }
        if len_m > 1 {
            assert_ne!(modulus.last().unwrap(), &U256::ZERO, "Modulus must not have leading zeros");
        } else {
            assert_ne!(modulus[0], U256::ZERO, "Modulus must not be zero");
        }
    }

    // If modulus == 1, then base^exp (mod 1) is always 0
    if len_m == 1 && modulus[0] == U256::ONE {
        return vec![U256::ZERO];
    }

    // If exp == 0, then base^0 (mod modulus) is 1
    if len_e == 1 && exp[0] == 0 {
        return vec![U256::ONE];
    }

    if len_b == 1 {
        // If base == 0, then 0^exp (mod modulus) is 0
        if base[0] == U256::ZERO {
            return vec![U256::ZERO];
        }

        // If base == 1, then 1^exp (mod modulus) is 1
        if base[0] == U256::ONE {
            return vec![U256::ONE];
        }
    }

    // We can assume from now on that base,modulus > 1 and exp > 0

    // Initialize out = 1
    let mut out = vec![U256::ZERO; len_m];
    out[0] = U256::ONE;

    // Compute base = base (mod modulus)
    let base = if U256::lt_slices_unchecked(base, modulus) {
        base.to_vec()
    } else if len_m == 1 {
        vec![rem_short(base, &modulus[0])]
    } else {
        rem_long(base, modulus)
    };

    // scratch space for intermediate computations
    let mut scratch = vec![U256::ZERO; 2 * len_m];
    for e in exp.iter().rev() {
        let mut mask: u64 = 1 << 63;
        while mask > 0 {
            // Compute out = out² (mod modulus);
            square_and_reduce(&out, modulus, &mut scratch);
            out.copy_from_slice(&scratch[..len_m]);
            scratch.fill(U256::ZERO);

            if e & mask != 0 {
                // Compute out = (out * base) (mod modulus);
                mul_and_reduce(&out, &base, modulus, &mut scratch);
                out.copy_from_slice(&scratch[..len_m]);
                scratch.fill(U256::ZERO);
            }
            mask >>= 1;
        }
    }

    // Strip any leading zeros at the end
    while out.len() > 1 && out.last() == Some(&U256::ZERO) {
        out.pop();
    }

    out
}

pub fn modexp_u64(base: &[u64], exp: &[u64], modulus: &[u64]) -> Vec<u64> {
    fn pad_to_multiple_of_4(input: &[u64]) -> Vec<u64> {
        let mut padded = input.to_vec();
        let remainder = input.len() % 4;
        if remainder != 0 {
            padded.resize(input.len() + (4 - remainder), 0u64);
        }
        padded
    }

    // Pad all inputs
    let padded_base = pad_to_multiple_of_4(base);
    let padded_modulus = pad_to_multiple_of_4(modulus);

    // Convert u64 arrays to U256 chunks
    let base_u256 = U256::slice_from_flat(&padded_base);
    let modulus_u256 = U256::slice_from_flat(&padded_modulus);

    // Call the main modexp function
    let result_u256 = modexp(&base_u256, &exp, &modulus_u256);

    // Convert result back to u64 array
    U256::slice_to_flat(&result_u256).to_vec()
}
