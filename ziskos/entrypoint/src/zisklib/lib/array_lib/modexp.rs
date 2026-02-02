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
pub fn modexp(
    base: &[U256],
    exp: &[u64],
    modulus: &[U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<U256> {
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

    // If modulus == 0, return zeros
    if len_m == 1 && modulus[0].is_zero() {
        return vec![U256::ZERO];
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
    if len_m == 1 {
        modexp_short(
            base,
            exp,
            &modulus[0],
            #[cfg(feature = "hints")]
            hints,
        )
    } else {
        modexp_long(
            base,
            exp,
            modulus,
            #[cfg(feature = "hints")]
            hints,
        )
    }
}

/// Short modexp when modulus fits in a single U256
fn modexp_short(
    base: &[U256],
    exp: &[u64],
    modulus: &U256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<U256> {
    let len_e = exp.len();

    // Compute base = base (mod modulus)
    let base = rem_short_init(
        base,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    // Hint exponent bits
    let (len, bits) = fcall_bin_decomp(
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

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
        out = square_and_reduce_short(
            &out,
            modulus,
            &mut scratch,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit == 1 {
            // Compute out = (out * base) (mod modulus)
            out = mul_and_reduce_short(
                &out,
                &base,
                modulus,
                &mut scratch,
                #[cfg(feature = "hints")]
                hints,
            );
            // Recompose the exponent
            let bits_pos = len - 1 - bit_idx;
            let limb_idx = bits_pos / 64;
            let bit_in_limb = bits_pos % 64;
            rec_exp[limb_idx] |= 1u64 << bit_in_limb;
        }
    }

    assert_eq!(rec_exp[..], *exp, "Exponent decomposition mismatch");

    vec![out]
}

/// Long modexp when modulus requires multiple U256 limbs
fn modexp_long(
    base: &[U256],
    exp: &[u64],
    modulus: &[U256],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Vec<U256> {
    let len_e = exp.len();
    let len_m = modulus.len();

    // Compute base = base (mod modulus)
    let base = rem_long_init(
        base,
        modulus,
        #[cfg(feature = "hints")]
        hints,
    );

    // Hint exponent bits
    let (len, bits) = fcall_bin_decomp(
        exp,
        #[cfg(feature = "hints")]
        hints,
    );

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
        out = square_and_reduce_long(
            &out,
            modulus,
            &mut scratch,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit == 1 {
            // Compute out = (out * base) (mod modulus)
            out = mul_and_reduce_long(
                &out,
                &base,
                modulus,
                &mut scratch,
                #[cfg(feature = "hints")]
                hints,
            );
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

/// Compute modular exponentiation from big-endian byte arrays
///
/// ### Safety
///
/// The caller must ensure that:
/// - `base_ptr` points to an array of `base_len` bytes (big-endian)
/// - `exp_ptr` points to an array of `exp_len` bytes (big-endian)
/// - `modulus_ptr` points to an array of `modulus_len` bytes (big-endian)
/// - `result_ptr` points to an array of at least `modulus_len` bytes
///
/// Returns the number of bytes written to `result_ptr` (always equals `modulus_len`, zero-padded)
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_modexp_bytes_c")]
pub unsafe extern "C" fn modexp_bytes_c(
    base_ptr: *const u8,
    base_len: usize,
    exp_ptr: *const u8,
    exp_len: usize,
    modulus_ptr: *const u8,
    modulus_len: usize,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> usize {
    let base_bytes = std::slice::from_raw_parts(base_ptr, base_len);
    let exp_bytes = std::slice::from_raw_parts(exp_ptr, exp_len);
    let modulus_bytes = std::slice::from_raw_parts(modulus_ptr, modulus_len);

    // Convert from big-endian bytes to little-endian u64/U256 arrays
    let base_u256 = bytes_be_to_u256_le(base_bytes);
    let exp_u64 = bytes_be_to_u64_le(exp_bytes);
    let modulus_u256 = bytes_be_to_u256_le(modulus_bytes);

    let result_u256 = modexp(
        &base_u256,
        &exp_u64,
        &modulus_u256,
        #[cfg(feature = "hints")]
        hints,
    );

    // Convert result back to big-endian bytes with proper length
    let result = std::slice::from_raw_parts_mut(result_ptr, modulus_len);
    u256_le_to_bytes_be(&result_u256, result);

    modulus_len
}

/// Convert big-endian bytes to little-endian u64 array
fn bytes_be_to_u64_le(bytes: &[u8]) -> Vec<u64> {
    if bytes.is_empty() {
        return vec![0];
    }

    // Skip leading zeros but keep at least one limb
    let first_nonzero = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len() - 1);
    let bytes = &bytes[first_nonzero..];

    if bytes.is_empty() {
        return vec![0];
    }

    // Process bytes into u64 limbs
    let num_limbs = bytes.len().div_ceil(8);
    let mut result = vec![0u64; num_limbs];
    for (i, &byte) in bytes.iter().rev().enumerate() {
        let limb_idx = i / 8;
        let byte_idx = i % 8;
        result[limb_idx] |= (byte as u64) << (byte_idx * 8);
    }

    result
}

/// Convert big-endian bytes to little-endian U256 array
fn bytes_be_to_u256_le(bytes: &[u8]) -> Vec<U256> {
    let u64_le = bytes_be_to_u64_le(bytes);

    // Pad to multiple of 4 u64s
    let padded_len = u64_le.len().next_multiple_of(4);
    let mut padded = vec![0u64; padded_len];
    padded[..u64_le.len()].copy_from_slice(&u64_le);

    U256::flat_to_slice(&padded).to_vec()
}

/// Convert little-endian U256 array to big-endian bytes
fn u256_le_to_bytes_be(limbs: &[U256], output: &mut [u8]) {
    let flat = U256::slice_to_flat(limbs);
    let out_len = output.len();
    output.fill(0);

    for (i, &limb) in flat.iter().enumerate() {
        for j in 0..8 {
            let byte_val = ((limb >> (j * 8)) & 0xFF) as u8;
            let pos_from_end = i * 8 + j;
            if pos_from_end < out_len {
                output[out_len - 1 - pos_from_end] = byte_val;
            }
        }
    }
}
