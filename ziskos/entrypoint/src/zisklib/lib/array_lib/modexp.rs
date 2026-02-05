// TODO: It can be speed up by using Montgomery multiplication but knowning that divisions are "free"
// For ref: https://www.microsoft.com/en-us/research/wp-content/uploads/1996/01/j37acmon.pdf

use std::vec;

use crate::zisklib::fcall_bin_decomp;

use super::{
    mul_and_reduce_long, mul_and_reduce_short, rem_long_init, rem_short_init,
    square_and_reduce_long, square_and_reduce_short, LongScratch, ShortScratch, U256,
};

/// Modular exponentiation of three large numbers
///
/// It assumes that modulus > 0 and len(base),len(exp),len(modulus) > 0
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
            assert!(!base[len_b - 1].is_zero(), "Base must not have leading zeros");
        }
        if len_e > 1 {
            assert_ne!(exp.last().unwrap(), &0, "Exponent must not have leading zeros");
        }
        if len_m > 1 {
            assert!(!modulus[len_m - 1].is_zero(), "Modulus must not have leading zeros");
        } else {
            assert!(!modulus[0].is_zero(), "Modulus must not be zero");
        }
    }

    // If modulus == 1, then base^exp (mod 1) is always 0
    if len_m == 1 && modulus[0].is_one() {
        return vec![U256::ZERO];
    }

    // If exp == 0, then base^0 (mod modulus) is 1
    if len_e == 1 && exp[0] == 0 {
        return vec![U256::ONE];
    }

    if len_b == 1 {
        // If base == 0, then 0^exp (mod modulus) is 0
        if base[0].is_zero() {
            return vec![U256::ZERO];
        }

        // If base == 1, then 1^exp (mod modulus) is 1
        if base[0].is_one() {
            return vec![U256::ONE];
        }
    }

    // We can assume from now on that base,modulus > 1 and exp > 0

    // There are two versions:
    //   - If len(modulus) == 1, we can use short reductions
    //   - If len(modulus) > 1, we must use long reductions
    if len_m == 1 {
        let modulus = &modulus[0];

        // Compute base = base (mod modulus)
        let base = rem_short_init(base, modulus);

        // Hint exponent bits
        let (len, bits) = fcall_bin_decomp(exp);

        // We should recompose the exponent from bits to verify correctness
        let mut rec_exp = vec![0u64; len_e];

        // Recompose the MSB
        let bits_pos = len - 1;
        let limb_idx = bits_pos / 64;
        let bit_in_limb = bits_pos % 64;
        rec_exp[limb_idx] = 1u64 << bit_in_limb;

        // Scratch space
        let mut scratch = ShortScratch::new();

        // Initialize out = base
        let mut out = base;
        for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
            if out.is_zero() {
                return vec![U256::ZERO];
            }

            // Compute out = out² (mod modulus)
            out = square_and_reduce_short(&out, modulus, &mut scratch);

            if bit == 1 {
                // Compute out = (out * base) (mod modulus);
                out = mul_and_reduce_short(&out, &base, modulus, &mut scratch);

                // Recompose the exponent
                let bits_pos = len - 1 - bit_idx;
                let limb_idx = bits_pos / 64;
                let bit_in_limb = bits_pos % 64;
                rec_exp[limb_idx] |= 1u64 << bit_in_limb;
            }
        }

        assert_eq!(rec_exp[..], *exp, "Exponent decomposition mismatch");

        vec![out]
    } else {
        // Compute base = base (mod modulus)
        let base = rem_long_init(base, modulus);

        // Hint exponent bits
        let (len, bits) = fcall_bin_decomp(exp);

        // We should recompose the exponent from bits to verify correctness
        let mut rec_exp = vec![0u64; len_e];

        // Recompose the MSB
        let bits_pos = len - 1;
        let limb_idx = bits_pos / 64;
        let bit_in_limb = bits_pos % 64;
        rec_exp[limb_idx] = 1u64 << bit_in_limb;

        // Scratch space
        let mut scratch = LongScratch::new(len_m);

        // Initialize out = base
        let mut out = base.clone();
        for (bit_idx, &bit) in bits.iter().enumerate().skip(1) {
            if out.len() == 1 && out[0].is_zero() {
                return vec![U256::ZERO];
            }

            // Compute out = out² (mod modulus)
            out = square_and_reduce_long(&out, modulus, &mut scratch);

            if bit == 1 {
                // Compute out = (out * base) (mod modulus);
                out = mul_and_reduce_long(&out, &base, modulus, &mut scratch);
                // Recompose the exponent
                let bits_pos = len - 1 - bit_idx;
                let limb_idx = bits_pos / 64;
                let bit_in_limb = bits_pos % 64;
                rec_exp[limb_idx] |= 1u64 << bit_in_limb;
            }
        }

        assert_eq!(rec_exp[..], *exp, "Exponent decomposition mismatch");

        out
    }
}

pub fn modexp_u64(base: &[u64], exp: &[u64], modulus: &[u64]) -> Vec<u64> {
    // Round up to multiple of 4
    let base_len = (base.len() + 3) & !3;
    let modulus_len = (modulus.len() + 3) & !3;

    let mut base_padded = vec![0u64; base_len];
    let mut modulus_padded = vec![0u64; modulus_len];

    base_padded[..base.len()].copy_from_slice(base);
    modulus_padded[..modulus.len()].copy_from_slice(modulus);

    // Convert u64 arrays to U256 chunks
    let base_u256 = U256::flat_to_slice(&base_padded);
    let modulus_u256 = U256::flat_to_slice(&modulus_padded);

    // Call the main modexp function
    let result_u256 = modexp(base_u256, exp, modulus_u256);

    // Convert result back to u64 array
    U256::slice_to_flat(&result_u256).to_vec()
}

/// Compute modular exponentiation of three large numbers
///
/// ### Safety
///
/// The caller must ensure that:
/// - `base_ptr` points to an array of `base_len` u64 elements
/// - `exp_ptr` points to an array of `exp_len` u64 elements
/// - `modulus_ptr` points to an array of `modulus_len` u64 elements
/// - `result_ptr` points to an array of at least `modulus_len` u64 elements
#[no_mangle]
pub unsafe extern "C" fn modexp_u64_c(
    base_ptr: *const u64,
    base_len: usize,
    exp_ptr: *const u64,
    exp_len: usize,
    modulus_ptr: *const u64,
    modulus_len: usize,
    result_ptr: *mut u64,
) -> usize {
    let base = std::slice::from_raw_parts(base_ptr, base_len);
    let exp = std::slice::from_raw_parts(exp_ptr, exp_len);
    let modulus = std::slice::from_raw_parts(modulus_ptr, modulus_len);

    // Round up to multiple of 4
    let base_len = (base.len() + 3) & !3;
    let modulus_len = (modulus.len() + 3) & !3;

    let mut base_padded = vec![0u64; base_len];
    let mut modulus_padded = vec![0u64; modulus_len];

    base_padded[..base.len()].copy_from_slice(base);
    modulus_padded[..modulus.len()].copy_from_slice(modulus);

    // Convert u64 arrays to U256 chunks
    let base_u256 = U256::flat_to_slice(&base_padded);
    let modulus_u256 = U256::flat_to_slice(&modulus_padded);

    // Call the main modexp function
    let result_u256 = modexp(base_u256, exp, modulus_u256);
    let result_slice = U256::slice_to_flat(&result_u256);
    let result_len = result_slice.len();

    // Convert result back to u64 array
    let result = std::slice::from_raw_parts_mut(result_ptr, modulus_len);
    result[..result_len].copy_from_slice(result_slice);

    result_len
}
