//! Integer witnesses for `precompiles/floatdot/pil/arithmetic.pil`.
//! Witness construction does not establish a proof; the AIR must check each row.

use crate::{fp32_add, fp32_mul};

pub const FP32_WITNESS_COLUMNS: usize = 1813;

/// One scalar AIR row, including the numeric path for exceptional inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fp32Witness {
    pub a: [u64; 32],
    pub b: [u64; 32],
    pub is_mul: u64,
    pub result: [u64; 32],
    pub flags: [u64; 5],
    pub fp_a_sig: [u64; 24],
    pub fp_a_sign: u64,
    pub fp_a_exp: u64,
    pub fp_a_cls: [u64; 6],
    pub fp_a_ez: [u64; 9],
    pub fp_a_eo: [u64; 9],
    pub fp_a_fz: [u64; 24],
    pub fp_b_sig: [u64; 24],
    pub fp_b_sign: u64,
    pub fp_b_exp: u64,
    pub fp_b_cls: [u64; 6],
    pub fp_b_ez: [u64; 9],
    pub fp_b_eo: [u64; 9],
    pub fp_b_fz: [u64; 24],
    pub fp_a_finite_sig: [u64; 24],
    pub fp_b_finite_sig: [u64; 24],
    pub fp_a_eff_exp: u64,
    pub fp_b_eff_exp: u64,
    pub fp_compare: [u64; 32],
    pub fp_delta: [u64; 8],
    pub fp_align_bits: [u64; 6],
    pub fp_align_stages: [[u64; 48]; 6],
    pub fp_align_sticky: [[u64; 33]; 6],
    pub fp_sign_xor: u64,
    pub fp_sum: [u64; 28],
    pub fp_product: [u64; 48],
    pub fp_norm_raw: [u64; 48],
    pub fp_norm_norm: [u64; 48],
    pub fp_norm_high_index: [u64; 6],
    pub fp_norm_prefix: [u64; 49],
    pub fp_norm_leading: [u64; 48],
    pub fp_norm_tail: [u64; 22],
    pub fp_norm_sig: [u64; 27],
    pub fp_exponent: [u64; 10],
    pub fp_sign: u64,
    pub fp_round_exp_lower: [u64; 10],
    pub fp_round_exp_upper: [u64; 10],
    pub fp_round_under_diff: [u64; 11],
    pub fp_round_tiny_diff: [u64; 11],
    pub fp_round_shift_amount: [u64; 8],
    pub fp_round_shift_bits: [u64; 6],
    pub fp_round_stages: [[u64; 48]; 6],
    pub fp_round_sticky: [[u64; 33]; 6],
    pub fp_round_rounded: [u64; 25],
    pub fp_round_inexact: u64,
    pub fp_round_top_ones: [u64; 26],
    pub fp_round_overflow_diff: [u64; 11],
    pub fp_round_result: [u64; 32],
    pub fp_round_flags: [u64; 5],
    pub fp_invalid: u64,
    pub fp_nan: u64,
    pub fp_inf: u64,
}

impl Fp32Witness {
    /// Construct both numeric branches, including the unused branch for NaN/Inf.
    pub fn new(a: u32, b: u32, is_mul: bool) -> Self {
        let da = Decoded::new(a);
        let db = Decoded::new(b);
        let abs_a = if da.exp == 255 { 0 } else { u64::from(a & 0x7fffffff) };
        let abs_b = if db.exp == 255 { 0 } else { u64::from(b & 0x7fffffff) };
        let (large, small) = if abs_a >= abs_b { (&da, &db) } else { (&db, &da) };
        let delta = large.eff_exp - small.eff_exp;
        let align = Shifted::new(small.finite_sig << 3, delta.min(63));
        let sign_xor = da.sign ^ db.sign;
        let sum = if sign_xor == 1 {
            (large.finite_sig << 3) - align.value
        } else {
            (large.finite_sig << 3) + align.value
        };
        let product = da.finite_sig * db.finite_sig;
        let raw = if is_mul { product } else { sum };
        let high = if raw == 0 { 0 } else { 63 - raw.leading_zeros() as u64 };
        let norm_raw: [u64; 48] = bits(raw);
        let norm: [u64; 48] = bits(raw << (47 - high));
        let mut prefix = [0; 49];
        let mut leading = [0; 48];
        prefix[48] = 1;
        for i in (0..48).rev() {
            leading[i] = norm_raw[i] * prefix[i + 1];
            prefix[i] = prefix[i + 1] * (1 - norm_raw[i]);
        }
        let mut tail = [0; 22];
        let mut sticky = 0;
        for i in 0..22 {
            sticky |= norm[i];
            tail[i] = sticky;
        }
        let sig = ((raw << (47 - high)) >> 21) | sticky;
        let exponent = if is_mul {
            da.eff_exp as i64 + db.eff_exp as i64 + high as i64 - 300
        } else {
            large.eff_exp as i64 + high as i64 - 153
        };
        let biased = (exponent + 512) as u64;
        let sign = if is_mul {
            sign_xor
        } else if sign_xor == 1 && sum == 0 {
            0
        } else {
            large.sign
        };
        let distance = (-126 - exponent).max(0) as u64;
        let round_shift = Shifted::new(sig, distance.min(63));
        let round_tail = round_shift.value & 7;
        let retained = round_shift.value >> 3;
        let rounded =
            retained + u64::from(round_tail > 4 || (round_tail == 4 && retained & 1 == 1));
        let carry = rounded >> 24;
        let overflow = sig != 0 && biased + carry >= 640;
        let tiny = distance > 0 && (exponent < -127 || sig < 0x7fffffc);
        let inexact = u64::from(round_tail != 0);
        let mut top_ones = [1; 26];
        for i in 0..25 {
            top_ones[i + 1] = top_ones[i] * ((sig >> (i + 2)) & 1);
        }
        let mantissa = rounded >> carry;
        let out_exp = if overflow {
            255
        } else if mantissa >= 0x800000 {
            (exponent.max(-126) + 127) as u64 + carry
        } else {
            0
        };
        let round_result =
            (sign << 31) | (out_exp << 23) | if overflow { 0 } else { mantissa & 0x7fffff };
        let round_flags = if overflow { 5 } else { inexact | ((u64::from(tiny) * inexact) << 1) };
        let any_nan = da.cls[4] + da.cls[5] + db.cls[4] + db.cls[5] != 0;
        let invalid = da.cls[5] + db.cls[5] != 0
            || if is_mul {
                da.cls[3] * db.cls[0] + db.cls[3] * da.cls[0] != 0
            } else {
                da.cls[3] * db.cls[3] * sign_xor != 0
            };
        let nan = any_nan || invalid;
        let inf = !nan && da.cls[3] + db.cls[3] != 0;
        let result = if is_mul { fp32_mul(a, b) } else { fp32_add(a, b) };
        Self {
            a: bits(a.into()),
            b: bits(b.into()),
            is_mul: is_mul.into(),
            result: bits(result.bits.into()),
            flags: bits(result.flags.into()),
            fp_a_sig: bits(da.sig),
            fp_a_sign: da.sign,
            fp_a_exp: da.exp,
            fp_a_cls: da.cls,
            fp_a_ez: da.ez,
            fp_a_eo: da.eo,
            fp_a_fz: da.fz,
            fp_b_sig: bits(db.sig),
            fp_b_sign: db.sign,
            fp_b_exp: db.exp,
            fp_b_cls: db.cls,
            fp_b_ez: db.ez,
            fp_b_eo: db.eo,
            fp_b_fz: db.fz,
            fp_a_finite_sig: bits(da.finite_sig),
            fp_b_finite_sig: bits(db.finite_sig),
            fp_a_eff_exp: da.eff_exp,
            fp_b_eff_exp: db.eff_exp,
            fp_compare: bits((1 << 31) + abs_a - abs_b),
            fp_delta: bits(delta),
            fp_align_bits: bits(delta.min(63)),
            fp_align_stages: align.stages,
            fp_align_sticky: align.sticky,
            fp_sign_xor: sign_xor,
            fp_sum: bits(sum),
            fp_product: bits(product),
            fp_norm_raw: norm_raw,
            fp_norm_norm: norm,
            fp_norm_high_index: bits(high),
            fp_norm_prefix: prefix,
            fp_norm_leading: leading,
            fp_norm_tail: tail,
            fp_norm_sig: bits(sig),
            fp_exponent: bits(biased),
            fp_sign: sign,
            fp_round_exp_lower: bits(biased - 214),
            fp_round_exp_upper: bits(767 - biased),
            fp_round_under_diff: bits(biased + 1024 - 386),
            fp_round_tiny_diff: bits(biased + 1024 - 385),
            fp_round_shift_amount: bits(distance),
            fp_round_shift_bits: bits(distance.min(63)),
            fp_round_stages: round_shift.stages,
            fp_round_sticky: round_shift.sticky,
            fp_round_rounded: bits(rounded),
            fp_round_inexact: inexact,
            fp_round_top_ones: top_ones,
            fp_round_overflow_diff: bits(biased + carry + 1024 - 640),
            fp_round_result: bits(round_result),
            fp_round_flags: bits(round_flags),
            fp_invalid: invalid.into(),
            fp_nan: nan.into(),
            fp_inf: inf.into(),
        }
    }

    /// Raw columns in `tests/arithmetic.pil` declaration order; matrices are row-major.
    pub fn to_row(&self) -> [u64; FP32_WITNESS_COLUMNS] {
        let columns: &[&[u64]] = &[
            &self.a,
            &self.b,
            &[self.is_mul],
            &self.result,
            &self.flags,
            &self.fp_a_sig,
            &[self.fp_a_sign],
            &[self.fp_a_exp],
            &self.fp_a_cls,
            &self.fp_a_ez,
            &self.fp_a_eo,
            &self.fp_a_fz,
            &self.fp_b_sig,
            &[self.fp_b_sign],
            &[self.fp_b_exp],
            &self.fp_b_cls,
            &self.fp_b_ez,
            &self.fp_b_eo,
            &self.fp_b_fz,
            &self.fp_a_finite_sig,
            &self.fp_b_finite_sig,
            &[self.fp_a_eff_exp],
            &[self.fp_b_eff_exp],
            &self.fp_compare,
            &self.fp_delta,
            &self.fp_align_bits,
            self.fp_align_stages.as_flattened(),
            self.fp_align_sticky.as_flattened(),
            &[self.fp_sign_xor],
            &self.fp_sum,
            &self.fp_product,
            &self.fp_norm_raw,
            &self.fp_norm_norm,
            &self.fp_norm_high_index,
            &self.fp_norm_prefix,
            &self.fp_norm_leading,
            &self.fp_norm_tail,
            &self.fp_norm_sig,
            &self.fp_exponent,
            &[self.fp_sign],
            &self.fp_round_exp_lower,
            &self.fp_round_exp_upper,
            &self.fp_round_under_diff,
            &self.fp_round_tiny_diff,
            &self.fp_round_shift_amount,
            &self.fp_round_shift_bits,
            self.fp_round_stages.as_flattened(),
            self.fp_round_sticky.as_flattened(),
            &self.fp_round_rounded,
            &[self.fp_round_inexact],
            &self.fp_round_top_ones,
            &self.fp_round_overflow_diff,
            &self.fp_round_result,
            &self.fp_round_flags,
            &[self.fp_invalid],
            &[self.fp_nan],
            &[self.fp_inf],
        ];
        let mut row = [0; FP32_WITNESS_COLUMNS];
        let mut offset = 0;
        for values in columns {
            row[offset..offset + values.len()].copy_from_slice(values);
            offset += values.len();
        }
        assert_eq!(offset, FP32_WITNESS_COLUMNS);
        row
    }
}

fn bits<const N: usize>(value: u64) -> [u64; N] {
    debug_assert!(N < 64 && value < 1 << N);
    std::array::from_fn(|i| (value >> i) & 1)
}

struct Decoded {
    sig: u64,
    sign: u64,
    exp: u64,
    cls: [u64; 6],
    ez: [u64; 9],
    eo: [u64; 9],
    fz: [u64; 24],
    finite_sig: u64,
    eff_exp: u64,
}

impl Decoded {
    fn new(value: u32) -> Self {
        let exp = u64::from((value >> 23) & 255);
        let fraction = u64::from(value & 0x7fffff);
        let sig = fraction | (u64::from(exp != 0) << 23);
        let mut ez = [1; 9];
        let mut eo = [1; 9];
        let mut fz = [1; 24];
        for i in 0..8 {
            ez[i + 1] = ez[i] * (1 - ((exp >> i) & 1));
            eo[i + 1] = eo[i] * ((exp >> i) & 1);
        }
        for i in 0..23 {
            fz[i + 1] = fz[i] * (1 - ((fraction >> i) & 1));
        }
        Self {
            sig,
            sign: u64::from(value >> 31),
            exp,
            cls: [
                exp == 0 && fraction == 0,
                exp == 0 && fraction != 0,
                exp > 0 && exp < 255,
                exp == 255 && fraction == 0,
                exp == 255 && fraction >= 0x400000,
                exp == 255 && fraction > 0 && fraction < 0x400000,
            ]
            .map(u64::from),
            ez,
            eo,
            fz,
            finite_sig: if exp == 255 { 0 } else { sig },
            eff_exp: if exp == 255 { 1 } else { exp.max(1) },
        }
    }
}

struct Shifted {
    value: u64,
    stages: [[u64; 48]; 6],
    sticky: [[u64; 33]; 6],
}

impl Shifted {
    fn new(mut value: u64, amount: u64) -> Self {
        debug_assert!(value < 1 << 48 && amount < 64);
        let mut stages = [[0; 48]; 6];
        let mut sticky = [[0; 33]; 6];
        for j in 0..6 {
            let distance = 1 << j;
            let mut any = 0;
            for (k, bit) in sticky[j].iter_mut().enumerate().take(distance + 1) {
                any |= (value >> k) & 1;
                *bit = any;
            }
            if amount & distance as u64 != 0 {
                value = (value >> distance) | u64::from(value & ((1 << distance) - 1) != 0);
            }
            stages[j] = bits(value);
        }
        Self { value, stages, sticky }
    }
}
