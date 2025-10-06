use std::vec;

use super::{div_long, div_short, halve, mul_long, mul_short, square, U256};

/// Modular exponentiation of three large numbers (represented as arrays of U256): base^exp (mod modulus)
///
/// It assumes that modulus > 0 and len(base),len(exp) > 0 (these are handled by the host library)
pub fn modexp(base: &[U256], exp: &[U256], modulus: &[U256]) -> Vec<U256> {
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
            assert_ne!(exp.last().unwrap(), &U256::ZERO, "Exponent must not have leading zeros");
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
    if len_e == 1 && exp[0] == U256::ZERO {
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

    // Reduce base modulo modulus once at the start
    let mut base = if len_b < len_m || (len_b == len_m && base < modulus) {
        base.to_vec()
    } else if len_m == 1 {
        vec![div_short(base, &modulus[0]).1]
    } else {
        div_long(base, modulus).1
    };
    let mut exp = exp.to_vec();

    let mut out = Vec::with_capacity(len_m);
    out.push(U256::ONE);
    while !(exp.len() == 1 && exp[0] == U256::ZERO) {
        if base.len() == 1 && base[0] == U256::ZERO {
            return vec![U256::ZERO];
        }

        // Is exp odd?
        // We only need to inspect the first chunk
        if exp[0].is_odd() {
            // Compute out = (out * base) (mod modulus);
            let mul =
                if base.len() == 1 { mul_short(&out, &base[0]) } else { mul_long(&out, &base) };

            let len_mul = mul.len();
            out = if len_mul < len_m || (len_mul == len_m && mul < modulus.to_vec()) {
                mul
            } else if len_m == 1 {
                vec![div_short(&mul, &modulus[0]).1]
            } else {
                div_long(&mul, modulus).1
            };
        }

        // Compute exp = exp / 2
        exp = halve(&exp);

        // Compute base = base² (mod modulus);
        let sq = square(&base);

        let len_sq = sq.len();
        base = if len_sq < len_m || (len_sq == len_m && sq < modulus.to_vec()) {
            sq
        } else if len_m == 1 {
            vec![div_short(&sq, &modulus[0]).1]
        } else {
            div_long(&sq, modulus).1
        };
    }

    out
}

/// Modular exponentiation of three large numbers (represented as arrays of u64): base^exp (mod modulus)
pub fn modexp_u64(base: &[u64], exp: &[u64], modulus: &[u64]) -> Vec<u64> {
    // Helper function to pad array to multiple of 4
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
    let padded_exp = pad_to_multiple_of_4(exp);
    let padded_modulus = pad_to_multiple_of_4(modulus);

    // Convert u64 arrays to U256 chunks
    let base_u256 = U256::slice_from_flat(&padded_base);
    let exp_u256 = U256::slice_from_flat(&padded_exp);
    let modulus_u256 = U256::slice_from_flat(&padded_modulus);

    // Call the main modexp function
    let result_u256 = modexp(&base_u256, &exp_u256, &modulus_u256);

    // Convert result back to u64 array
    U256::slice_to_flat(&result_u256).to_vec()
}
