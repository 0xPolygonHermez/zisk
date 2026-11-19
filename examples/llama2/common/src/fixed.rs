//! Fixed-point scales for the matrix multiplications.
//!
//! In llama2.c, each group of a matrix multiplication adds
//! `ival · w_scale · x_scale` to an `f32` sum, which on ZisK costs four
//! software-float calls per group. In fixed point, the scales of a matrix
//! row, and those of a quantized vector, become integers that share one
//! power-of-two exponent:
//!
//! ```text
//! w_scale[g] ≈ w_fixed[g] · 2^-w_shift      x_scale[g] ≈ x_fixed[g] · 2^-x_shift
//! out = 2^-(w_shift + x_shift) · Σ_g ival[g] · w_fixed[g] · x_fixed[g]
//! ```
//!
//! The sum is exact in `i64`, and only the final value becomes an `f32`.
//!
//! The exponent is chosen so that the largest scale becomes an integer of
//! [`SCALE_BITS`] bits, so the other scales keep a precision of at least
//! `2^-SCALE_BITS` of the largest one. That is far below the int8
//! quantization error, about `2^-8` of each group's largest value.
//!
//! All conversions work on the bits of the `f32` values, with integer
//! instructions only, so they are cheap on ZisK and give the same result
//! natively.

/// Bits of a fixed-point scale: the largest scale of a row, or of a vector,
/// becomes an integer in `[2^14, 2^15]`.
pub const SCALE_BITS: i32 = 15;

/// The largest number of values in a matrix row or a vector for which the
/// fixed-point sum cannot overflow an `i64`: each term is at most
/// `127² · 2^(2·SCALE_BITS) < 2^44` per value.
pub const MAX_LEN: usize = 1 << 19;

/// Splits a finite `f32` that is 0 or more into `(m, e)`, with
/// `value = m · 2^e` exactly and `m < 2^24`.
fn split(bits: u32) -> (u64, i32) {
    let exp = (bits >> 23) as i32 & 0xff;
    let frac = (bits & 0x7f_ffff) as u64;
    if exp == 0 { (frac, -149) } else { (frac | 0x80_0000, exp - 150) }
}

/// The exponent `k` such that `max · 2^k` is in `[2^(SCALE_BITS-1),
/// 2^SCALE_BITS)`, from the bits of `max`, the largest of the scales that
/// share it. Positive `f32` values compare like their bits, so `max` can be
/// found with integer comparisons. Returns 0 for a zero `max`.
pub fn shift_for(max_bits: u32) -> i32 {
    if max_bits == 0 {
        return 0;
    }
    let (m, e) = split(max_bits);
    let msb = 63 - m.leading_zeros() as i32;
    SCALE_BITS - 1 - msb - e
}

/// `round(value · 2^k)` for a finite `f32` value that is 0 or more, given as
/// bits, with ties rounded up. The caller chooses `k` with [`shift_for`] from
/// a value at least as large, so the result fits in `SCALE_BITS` bits.
pub fn to_fixed(bits: u32, k: i32) -> i32 {
    let (m, e) = split(bits);
    let s = e + k;
    let v = if s >= 0 {
        m << s
    } else if s < -40 {
        0
    } else {
        (m + (1 << (-s - 1))) >> -s
    };
    v as i32
}

/// Converts the scales that share one exponent to fixed point. Returns that
/// exponent.
pub fn scales_to_fixed(scales: &[f32], fixed: &mut [i32]) -> i32 {
    let max = scales.iter().fold(0, |m, s| m.max(s.to_bits()));
    let k = shift_for(max);
    for (f, s) in fixed.iter_mut().zip(scales) {
        *f = to_fixed(s.to_bits(), k);
    }
    k
}

/// `acc · 2^-k` as an `f32`. The conversion of `acc` is correctly rounded,
/// and the scaling only changes the exponent, so it is exact unless the
/// result is below 2^-126 (it becomes 0) or too large (it becomes infinite);
/// neither happens with these scales.
pub fn fixed_to_f32(acc: i64, k: i32) -> f32 {
    scale_f32(acc as f32, k)
}

/// [`fixed_to_f32`] for an `i128` sum. The FP8 and FP4 formats need it: their
/// dot products are larger integers (in units of 2^-18 for two E4M3 values),
/// so with the scales applied a row's sum no longer fits an `i64`.
pub fn wide_fixed_to_f32(acc: i128, k: i32) -> f32 {
    scale_f32(acc as f32, k)
}

/// `f · 2^-k`, changing only the exponent of `f` (see [`fixed_to_f32`]).
fn scale_f32(f: f32, k: i32) -> f32 {
    let bits = f.to_bits();
    let exp = (bits >> 23) as i32 & 0xff;
    if exp == 0 {
        return f;
    }
    let exp = exp - k;
    let sign = bits & 0x8000_0000;
    if exp <= 0 {
        f32::from_bits(sign)
    } else if exp >= 0xff {
        f32::from_bits(sign | 0x7f80_0000)
    } else {
        f32::from_bits(sign | (exp as u32) << 23 | (bits & 0x7f_ffff))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largest_scale_uses_scale_bits() {
        for s in [1.0e-6f32, 3.7e-3, 0.5, 1.0, 12.25, 3.0e5] {
            let k = shift_for(s.to_bits());
            let f = to_fixed(s.to_bits(), k);
            assert!((1 << (SCALE_BITS - 1)..=1 << SCALE_BITS).contains(&f), "{s}: {f}");
            let back = f as f64 / libm::pow(2.0, k as f64);
            assert!(((back - s as f64) / s as f64).abs() < 1.0 / (1 << SCALE_BITS) as f64);
        }
    }

    #[test]
    fn to_fixed_rounds_to_nearest() {
        // 2.5 · 2^0 and 0.75 · 2^1 round half up; 0.3 · 2^4 = 4.8.
        assert_eq!(to_fixed(2.5f32.to_bits(), 0), 3);
        assert_eq!(to_fixed(0.75f32.to_bits(), 1), 2);
        assert_eq!(to_fixed(0.3f32.to_bits(), 4), 5);
        assert_eq!(to_fixed(0.0f32.to_bits(), 30), 0);
        assert_eq!(to_fixed(1.0e-30f32.to_bits(), 10), 0);
    }

    #[test]
    fn fixed_to_f32_scales_by_a_power_of_two() {
        assert_eq!(fixed_to_f32(3, 1), 1.5);
        assert_eq!(fixed_to_f32(-3 << 20, 40), -3.0 / (1u64 << 20) as f32);
        assert_eq!(fixed_to_f32(0, 40), 0.0);
        assert_eq!(fixed_to_f32(1, 200), 0.0);
    }

    #[test]
    fn subnormal_scales() {
        let s = f32::from_bits(5); // 5 · 2^-149
        let k = shift_for(s.to_bits());
        assert_eq!(to_fixed(s.to_bits(), k), 5 << (SCALE_BITS - 3));
    }
}
