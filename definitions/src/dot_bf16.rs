//! Reference semantics of the experimental `dot_bf16` precompile (see
//! `precompiles/dot_bf16/README.md`).
//!
//! A BF16 value is 1 sign bit, 8 exponent bits (bias 127) and 7 mantissa bits: the upper half of
//! an `f32`. `dot_bf16` computes `Σ x[i]·y[i]` over two BF16 vectors **exactly**, then rounds the
//! sum once to `f32`, to nearest, ties to even:
//!
//! - Every product is exact: two 8-bit significands make at most 16 bits, at a binary exponent
//!   from -266 to 240. The sum is kept in a 576-bit two's complement fixed-point accumulator
//!   ([`Bf16Accumulator`]) whose least significant bit is 2^-266, so there is no intermediate
//!   rounding or overflow, and the result does not depend on the order of the products.
//! - BF16 subnormals are kept, and the final rounding produces `f32` subnormals, ±∞ on overflow
//!   and 0 on underflow, as IEEE 754 does.
//! - Departures from IEEE 754, chosen to keep the operation simple to prove: an exact sum of
//!   zero gives +0.0 whatever the signs of the products, and if any input value is ±∞ or NaN the
//!   result is the canonical quiet NaN [`DOT_BF16_NAN`].
//!
//! With blocks, the result of each block is its own exact sum rounded once (NaN if the block
//! holds a ±∞ or NaN), and the total is the exact sum of all the products rounded once, which is
//! not the sum of the rounded blocks.

/// The largest vector length a call accepts. It bounds the data a single step carries, and the
/// accumulator has room for far more.
pub const DOT_BF16_MAX_N: u64 = 1 << 20;

/// The `f32` bits of the result when an input value is ±∞ or NaN: the canonical quiet NaN.
pub const DOT_BF16_NAN: u32 = 0x7FC0_0000;

/// Words of the accumulator: 576 bits, from 2^-266 up to a sign bit at 2^309.
const WORDS: usize = 9;

/// Binary exponent of the accumulator's least significant bit: the smallest product, of two
/// smallest BF16 subnormals (2^-133 each).
const LSB_EXP: i32 = -266;

/// Accumulator bit of 2^-149, the smallest `f32` subnormal.
const F32_MIN_SUBNORMAL_BIT: i32 = -149 - LSB_EXP;

/// A finite BF16 value as `(negative, significand, exponent)`, with value
/// `(-1)^negative · significand · 2^exponent` and `significand < 256`. `None` for ±∞ and NaN.
pub fn bf16_decode(b: u16) -> Option<(bool, u32, i32)> {
    let neg = b & 0x8000 != 0;
    let e = ((b >> 7) & 0xff) as i32;
    let m = (b & 0x7f) as u32;
    match e {
        0xff => None,
        0 => Some((neg, m, 1 - 127 - 7)),
        _ => Some((neg, 0x80 | m, e - 127 - 7)),
    }
}

/// The `f32` equal to a BF16 value: exact, the BF16 bits followed by 16 zero bits.
pub fn bf16_to_f32(b: u16) -> f32 {
    f32::from_bits((b as u32) << 16)
}

/// The BF16 nearest to an `f32`, ties to even. Values that round past the largest BF16 become
/// ±∞; a NaN stays a NaN (made quiet), whatever its payload.
pub fn bf16_from_f32(f: f32) -> u16 {
    let x = f.to_bits();
    if x & 0x7fff_ffff > 0x7f80_0000 {
        return ((x >> 16) | 0x0040) as u16;
    }
    let rounded = x as u64 + 0x7fff + ((x >> 16) & 1) as u64;
    (rounded >> 16) as u16
}

/// An exact sum of BF16 products: a 576-bit two's complement fixed-point number whose least
/// significant bit is 2^-266.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bf16Accumulator {
    words: [u64; WORDS],
}

impl Bf16Accumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the exact product of two finite values given as `(negative, significand, exponent)`
    /// parts: two BF16 values ([`bf16_decode`]), or an FP4 weight ([`fp4_weight_decode`]) and
    /// a BF16 value. The product's significand is below 2^30 and its value below 2^260, and its
    /// last bit is at most 2^-266, so it fits the accumulator.
    pub fn add_product(&mut self, a: (bool, u32, i32), b: (bool, u32, i32)) {
        let sig = a.1 as u64 * b.1 as u64; // below 2^30
        if sig == 0 {
            return;
        }
        let pos = (a.2 + b.2 - LSB_EXP) as usize; // 0..=512 for these formats
        let (w, off) = (pos / 64, pos % 64);
        let wide = (sig as u128) << off;
        let parts = [wide as u64, (wide >> 64) as u64];
        debug_assert!(w + 1 < WORDS || parts[1] == 0, "product out of the accumulator's range");
        if a.0 != b.0 {
            self.sub_at(w, parts);
        } else {
            self.add_at(w, parts);
        }
    }

    fn add_at(&mut self, w: usize, parts: [u64; 2]) {
        let mut carry = false;
        for i in w..WORDS {
            let p = if i - w < 2 { parts[i - w] } else { 0 };
            let (s1, c1) = self.words[i].overflowing_add(p);
            let (s2, c2) = s1.overflowing_add(carry as u64);
            self.words[i] = s2;
            carry = c1 || c2;
            if !carry && i > w {
                break;
            }
        }
    }

    fn sub_at(&mut self, w: usize, parts: [u64; 2]) {
        let mut borrow = false;
        for i in w..WORDS {
            let p = if i - w < 2 { parts[i - w] } else { 0 };
            let (s1, b1) = self.words[i].overflowing_sub(p);
            let (s2, b2) = s1.overflowing_sub(borrow as u64);
            self.words[i] = s2;
            borrow = b1 || b2;
            if !borrow && i > w {
                break;
            }
        }
    }

    /// Adds another accumulator.
    pub fn add(&mut self, other: &Bf16Accumulator) {
        let mut carry = false;
        for i in 0..WORDS {
            let (s1, c1) = self.words[i].overflowing_add(other.words[i]);
            let (s2, c2) = s1.overflowing_add(carry as u64);
            self.words[i] = s2;
            carry = c1 || c2;
        }
    }

    pub fn is_zero(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// The sum rounded to `f32`, to nearest, ties to even, as `f32` bits. A zero sum gives +0.0.
    pub fn to_f32_bits(&self) -> u32 {
        let neg = self.words[WORDS - 1] >> 63 != 0;
        let mag = if neg { negate(&self.words) } else { self.words };
        let Some(top) = highest_bit(&mag) else {
            return 0;
        };
        // Keep 24 significant bits, or fewer for an f32 subnormal, whose last bit is 2^-149.
        let mut lsb = (top - 23).max(F32_MIN_SUBNORMAL_BIT);
        let mut q = bits(&mag, lsb, (top - lsb + 1) as u32);
        let half = lsb > 0 && bit(&mag, lsb - 1);
        let sticky = lsb > 1 && any_below(&mag, lsb - 1);
        if half && (sticky || q & 1 == 1) {
            q += 1;
            if q == 1 << 24 {
                q = 1 << 23;
                lsb += 1;
            }
        }
        let sign = (neg as u32) << 31;
        if q < 1 << 23 {
            // Subnormal (or zero after rounding a value below 2^-150): lsb is 2^-149
            return sign | q as u32;
        }
        let biased = (lsb + LSB_EXP) + 23 + 127;
        if biased >= 0xff {
            return sign | 0x7f80_0000;
        }
        sign | (biased as u32) << 23 | (q as u32 & 0x7f_ffff)
    }
}

fn negate(w: &[u64; WORDS]) -> [u64; WORDS] {
    let mut r = [0u64; WORDS];
    let mut carry = true;
    for i in 0..WORDS {
        let (s, c) = (!w[i]).overflowing_add(carry as u64);
        r[i] = s;
        carry = c;
    }
    r
}

fn highest_bit(w: &[u64; WORDS]) -> Option<i32> {
    (0..WORDS)
        .rev()
        .find(|&i| w[i] != 0)
        .map(|i| (i * 64 + 63 - w[i].leading_zeros() as usize) as i32)
}

fn bit(w: &[u64; WORDS], pos: i32) -> bool {
    let pos = pos as usize;
    (w[pos / 64] >> (pos % 64)) & 1 == 1
}

/// `count` bits (at most 25) of `w` starting at bit `pos`.
fn bits(w: &[u64; WORDS], pos: i32, count: u32) -> u64 {
    let pos = pos as usize;
    let (i, off) = (pos / 64, pos % 64);
    let lo = w[i] >> off;
    let hi = if off > 0 && i + 1 < WORDS { w[i + 1] << (64 - off) } else { 0 };
    (lo | hi) & ((1u64 << count) - 1)
}

/// Whether any bit of `w` below bit `pos` is set.
fn any_below(w: &[u64; WORDS], pos: i32) -> bool {
    let pos = pos as usize;
    let (i, off) = (pos / 64, pos % 64);
    w[..i].iter().any(|&x| x != 0) || (off > 0 && w[i] & ((1u64 << off) - 1) != 0)
}

/// The reference semantics of `dot_bf16`: returns the `f32` bits of `Σ x[i]·y[i]` over the
/// first `n` values, computed exactly and rounded once, and writes the result of each block of
/// `block` values to `out` when given (`block` = 0 means one block of `n` values). Checks nothing
/// else: the callers validate the parameters.
pub fn dot_bf16_reference(
    x: &[u16],
    y: &[u16],
    n: usize,
    block: usize,
    mut out: Option<&mut [u32]>,
) -> u32 {
    let block = if block == 0 { n.max(1) } else { block };
    let mut total = Bf16Accumulator::new();
    let mut total_special = false;
    for (b, start) in (0..n).step_by(block).enumerate() {
        let mut acc = Bf16Accumulator::new();
        let mut special = false;
        for i in start..(start + block).min(n) {
            match (bf16_decode(x[i]), bf16_decode(y[i])) {
                (Some(a), Some(c)) => acc.add_product(a, c),
                _ => special = true,
            }
        }
        if let Some(out) = out.as_deref_mut() {
            out[b] = if special { DOT_BF16_NAN } else { acc.to_f32_bits() };
        }
        total.add(&acc);
        total_special |= special;
    }
    if total_special {
        DOT_BF16_NAN
    } else {
        total.to_f32_bits()
    }
}

// ----- FP4 weights with BF16 activations: MXFP4 (flags = 1) and NVFP4 (flags = 2) -----
//
// The weight vector holds E2M1 values (4 bits, two per byte, low nibble first: ±{0, 0.5, 1,
// 1.5, 2, 3, 4, 6}), with one scale byte per block of values:
//
// - MXFP4 (OCP Microscaling): blocks of 32, E8M0 scales: 2^(s - 127), and 0xFF is NaN.
// - NVFP4 (NVIDIA): blocks of 16, E4M3 (OCP FP8) scales: up to 448, and 0x7F/0xFF are NaN.
//   NVFP4 also has an f32 scale per tensor, which the caller applies to the result.
//
// The weight `e2m1(x[i]) · scale[i / block]` times the BF16 `y[i]` is exact, so the result is
// again the exact sum rounded once to f32, with the same rules as for two BF16 vectors. A NaN
// scale, like an infinite or NaN BF16 value, makes the result the canonical NaN.

/// `flags` of a `dot_bf16` call with two BF16 vectors.
pub const DOT_BF16_FLAGS_BF16: u64 = 0;
/// `flags` of a `dot_bf16` call with MXFP4 weights (E2M1, E8M0 scale per 32) and BF16 values.
pub const DOT_BF16_FLAGS_MXFP4: u64 = 1;
/// `flags` of a `dot_bf16` call with NVFP4 weights (E2M1, E4M3 scale per 16) and BF16 values.
pub const DOT_BF16_FLAGS_NVFP4: u64 = 2;

/// Values per scale of the FP4 weights of `flags`: 32 for MXFP4, 16 for NVFP4.
pub const fn dot_bf16_fp4_block(flags: u64) -> usize {
    match flags {
        DOT_BF16_FLAGS_MXFP4 => 32,
        DOT_BF16_FLAGS_NVFP4 => 16,
        _ => panic!("dot_bf16: not an FP4 flag"),
    }
}

/// The E2M1 value of a nibble, in units of 2^-1 and signed.
fn e2m1_units(nibble: u8) -> i32 {
    crate::DOT_F8_DECODE[crate::DOT_F8_E2M1 as usize][(nibble & 15) as usize]
}

/// The FP4 weight `e2m1(code) · scale` as `(negative, significand, exponent)`, with value
/// `(-1)^negative · significand · 2^exponent`; `None` for a NaN scale. `flags` selects the scale
/// format: E8M0 (MXFP4) or E4M3 (NVFP4).
pub fn fp4_weight_decode(flags: u64, code: u8, scale: u8) -> Option<(bool, u32, i32)> {
    let v = e2m1_units(code);
    match flags {
        DOT_BF16_FLAGS_MXFP4 => {
            if scale == 0xff {
                return None;
            }
            Some((v < 0, v.unsigned_abs(), scale as i32 - 127 - 1))
        }
        DOT_BF16_FLAGS_NVFP4 => {
            if scale & 0x7f == 0x7f {
                return None;
            }
            let s = crate::DOT_F8_DECODE[crate::DOT_F8_E4M3 as usize][scale as usize];
            Some(((v < 0) != (s < 0), v.unsigned_abs() * s.unsigned_abs(), -1 - 9))
        }
        _ => panic!("dot_bf16: not an FP4 flag"),
    }
}

/// The E2M1 code of value `i` of a packed FP4 vector.
pub fn fp4_code(x: &[u8], i: usize) -> u8 {
    let b = x[i / 2];
    if i & 1 == 0 {
        b & 15
    } else {
        b >> 4
    }
}

/// The reference semantics of `dot_bf16` with FP4 weights (`flags` = [`DOT_BF16_FLAGS_MXFP4`]
/// or [`DOT_BF16_FLAGS_NVFP4`]): returns the `f32` bits of `Σ w[i]·y[i]` over `n` values, the
/// weight `w[i] = e2m1(x[i]) · scale[i / fp4 block]`, computed exactly and rounded once, and
/// writes the result of each block of `block` values to `out` when given (0 = one block of `n`).
/// Checks nothing else: the callers validate the parameters.
pub fn dot_bf16_fp4_reference(
    flags: u64,
    x: &[u8],
    scales: &[u8],
    y: &[u16],
    n: usize,
    block: usize,
    mut out: Option<&mut [u32]>,
) -> u32 {
    let fp4_block = dot_bf16_fp4_block(flags);
    let block = if block == 0 { n.max(1) } else { block };
    let mut total = Bf16Accumulator::new();
    let mut total_special = false;
    for (b, start) in (0..n).step_by(block).enumerate() {
        let mut acc = Bf16Accumulator::new();
        let mut special = false;
        for i in start..(start + block).min(n) {
            match (
                fp4_weight_decode(flags, fp4_code(x, i), scales[i / fp4_block]),
                bf16_decode(y[i]),
            ) {
                (Some(w), Some(v)) => acc.add_product(w, v),
                _ => special = true,
            }
        }
        if let Some(out) = out.as_deref_mut() {
            out[b] = if special { DOT_BF16_NAN } else { acc.to_f32_bits() };
        }
        total.add(&acc);
        total_special |= special;
    }
    if total_special {
        DOT_BF16_NAN
    } else {
        total.to_f32_bits()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec;
    use std::vec::Vec;

    /// 2^k as a BF16, for k in -133..=127.
    fn pow2(k: i32) -> u16 {
        assert!((-133..=127).contains(&k));
        if k >= -126 {
            ((k + 127) as u16) << 7
        } else {
            1 << (k + 133)
        }
    }

    fn neg(b: u16) -> u16 {
        b ^ 0x8000
    }

    /// The BF16 equal to `v`, which must be exactly representable.
    fn bf(v: f32) -> u16 {
        let b = bf16_from_f32(v);
        assert_eq!(bf16_to_f32(b).to_bits(), v.to_bits(), "{v} is not a BF16");
        b
    }

    const ONE: u16 = 0x3f80;
    const MAX: u16 = 0x7f7f;
    const INF: u16 = 0x7f80;

    fn dot(x: &[u16], y: &[u16]) -> u32 {
        dot_bf16_reference(x, y, x.len(), 0, None)
    }

    #[test]
    fn helpers() {
        assert_eq!(pow2(0), ONE);
        assert_eq!(bf16_to_f32(pow2(-133)), f32::from_bits(1 << 16));
        assert_eq!(bf16_to_f32(pow2(-133)) as f64, 2f64.powi(-133));
        assert_eq!(bf16_to_f32(pow2(100)) as f64, 2f64.powi(100));
        assert_eq!(bf(10.0), 0x4120);
    }

    // The basic tests of precompiles/dot_bf16/README.md, numbered as there.

    #[test]
    fn t01_simple_sum() {
        let x = [bf(1.0), bf(2.0), bf(3.0), bf(4.0)];
        assert_eq!(dot(&x, &[ONE; 4]), 0x4120_0000);
    }

    #[test]
    fn t02_exact_cancellation() {
        let x = [pow2(100), ONE, neg(pow2(100)), 0];
        let y = [ONE, ONE, ONE, 0];
        assert_eq!(dot(&x, &y), 0x3f80_0000);
        // An f32 sum in this order loses the 1
        let f = |b: u16| bf16_to_f32(b);
        assert_eq!(f(x[0]) + f(x[1]) + f(x[2]), 0.0);
    }

    #[test]
    fn t03_final_tie_to_even_down() {
        assert_eq!(dot(&[ONE, pow2(-24), 0, 0], &[ONE, ONE, 0, 0]), 0x3f80_0000);
    }

    #[test]
    fn t04_final_tie_to_even_up() {
        let x = [ONE, pow2(-24), pow2(-23), 0];
        assert_eq!(dot(&x, &[ONE, ONE, ONE, 0]), 0x3f80_0002);
    }

    #[test]
    fn t05_sticky_bit() {
        let x = [ONE, pow2(-24), pow2(-60), 0];
        assert_eq!(dot(&x, &[ONE, ONE, ONE, 0]), 0x3f80_0001);
    }

    #[test]
    fn t06_underflow_to_zero() {
        assert_eq!(dot(&[pow2(-133), 0, 0, 0], &[pow2(-133), 0, 0, 0]), 0);
    }

    #[test]
    fn t07_smallest_f32_subnormal() {
        assert_eq!(dot(&[pow2(-75), 0, 0, 0], &[pow2(-74), 0, 0, 0]), 1);
    }

    #[test]
    fn t08_overflow() {
        assert_eq!(dot(&[MAX, 0, 0, 0], &[bf(2.0), 0, 0, 0]), 0x7f80_0000);
        assert_eq!(dot(&[neg(MAX), 0, 0, 0], &[bf(2.0), 0, 0, 0]), 0xff80_0000);
    }

    #[test]
    fn t09_no_intermediate_overflow() {
        let x = [MAX, MAX, neg(MAX), 0];
        let y = [bf(2.0), ONE, bf(2.0), 0];
        assert_eq!(dot(&x, &y), 0x7f7f_0000);
    }

    #[test]
    fn t10_zero_is_positive() {
        assert_eq!(dot(&[neg(ONE), 0, 0, 0], &[0, 0, 0, 0]), 0);
        assert_eq!(dot(&[neg(ONE), ONE, 0, 0], &[ONE, ONE, 0, 0]), 0);
    }

    #[test]
    fn t11_infinity_input() {
        assert_eq!(dot(&[INF, ONE, ONE, ONE], &[0, ONE, ONE, ONE]), DOT_BF16_NAN);
        assert_eq!(dot(&[ONE, ONE, ONE, ONE], &[ONE, neg(INF), ONE, ONE]), DOT_BF16_NAN);
    }

    #[test]
    fn t12_nan_input() {
        assert_eq!(dot(&[0x7fc1, 0, 0, 0], &[ONE, 0, 0, 0]), DOT_BF16_NAN);
    }

    #[test]
    fn t13_blocks() {
        let x = [pow2(100), 0, 0, 0, neg(pow2(100)), ONE, 0, 0];
        let mut out = [0u32; 2];
        let c = dot_bf16_reference(&x, &[ONE; 8], 8, 4, Some(&mut out));
        assert_eq!(out, [0x7180_0000, 0xf180_0000]); // ±2^100
        assert_eq!(c, 0x3f80_0000);
        // A NaN in one block only affects that block and the total
        let x = [ONE, 0, 0, 0, INF, 0, 0, 0];
        let c = dot_bf16_reference(&x, &[ONE; 8], 8, 4, Some(&mut out));
        assert_eq!((out, c), ([0x3f80_0000, DOT_BF16_NAN], DOT_BF16_NAN));
    }

    #[test]
    fn t14_empty() {
        assert_eq!(dot_bf16_reference(&[], &[], 0, 0, None), 0);
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    /// A random BF16 with an unbiased exponent in `lo..=hi`.
    fn random_bf16(r: &mut Rng, lo: i32, hi: i32) -> u16 {
        let e = lo + (r.next() % (hi - lo + 1) as u64) as i32;
        let m = (r.next() & 0x7f) as u16;
        let s = ((r.next() & 1) as u16) << 15;
        s | ((e + 127) as u16) << 7 | m
    }

    /// Exact oracle for values with exponents in [-8, 8]: every product times 2^30 is an
    /// integer, so the sum is exact in i128; `as f32` rounds to nearest even, and the scaling by
    /// 2^-30 is exact for these magnitudes.
    fn oracle(x: &[u16], y: &[u16]) -> u32 {
        let mut sum: i128 = 0;
        for (&a, &b) in x.iter().zip(y) {
            let (na, sa, ea) = bf16_decode(a).unwrap();
            let (nb, sb, eb) = bf16_decode(b).unwrap();
            let p = ((sa * sb) as i128) << (ea + eb + 30);
            sum += if na != nb { -p } else { p };
        }
        if sum == 0 {
            return 0;
        }
        let f = sum as f32;
        (f * 2f32.powi(-30)).to_bits()
    }

    #[test]
    fn t15_random_against_exact_oracle() {
        let mut r = Rng(0x9e37_79b9_7f4a_7c15);
        for _ in 0..10_000 {
            let n = 4 * (1 + (r.next() % 1024) as usize);
            let x: Vec<u16> = (0..n).map(|_| random_bf16(&mut r, -8, 8)).collect();
            let y: Vec<u16> = (0..n).map(|_| random_bf16(&mut r, -8, 8)).collect();
            assert_eq!(dot(&x, &y), oracle(&x, &y), "n = {n}");
        }
    }

    #[test]
    fn order_does_not_matter_over_the_full_range() {
        let mut r = Rng(0x1234_5678_9abc_def1);
        for _ in 0..2_000 {
            let n = 4 * (1 + (r.next() % 64) as usize);
            let mut x: Vec<u16> = (0..n).map(|_| random_bf16(&mut r, -126, 127)).collect();
            let mut y: Vec<u16> = (0..n).map(|_| random_bf16(&mut r, -126, 127)).collect();
            let c = dot(&x, &y);
            // Reverse, then rotate: different orders of the same products
            x.reverse();
            y.reverse();
            assert_eq!(dot(&x, &y), c);
            let k = (r.next() % n as u64) as usize;
            x.rotate_left(k);
            y.rotate_left(k);
            assert_eq!(dot(&x, &y), c);
        }
    }

    #[test]
    fn single_products_round_like_f64() {
        // A single product is exact in f64 (16-bit significand), and f64 -> f32 rounds to nearest
        // even, so it is an oracle over the full range, subnormals and overflow included.
        let mut r = Rng(0xdead_beef_cafe_f00d);
        for _ in 0..200_000 {
            let a = (r.next() as u16) & 0x7fff | ((r.next() as u16) & 0x8000);
            let b = (r.next() as u16) & 0x7fff | ((r.next() as u16) & 0x8000);
            if bf16_decode(a).is_none() || bf16_decode(b).is_none() {
                continue;
            }
            let p = bf16_to_f32(a) as f64 * bf16_to_f32(b) as f64;
            let expected = if p == 0.0 { 0 } else { (p as f32).to_bits() };
            assert_eq!(dot(&[a, 0, 0, 0], &[b, 0, 0, 0]), expected, "{a:#06x} * {b:#06x}");
        }
    }

    #[test]
    fn bf16_to_f32_round_trips_every_code() {
        for b in 0..=u16::MAX {
            let f = bf16_to_f32(b);
            if f.is_nan() {
                assert!(bf16_from_f32(f) & 0x7fff > 0x7f80, "{b:#06x}");
            } else {
                assert_eq!(bf16_from_f32(f), b, "{b:#06x}");
            }
        }
    }

    #[test]
    fn bf16_from_f32_rounds_to_nearest_even() {
        // 1 + 2^-8 is halfway between 1 and 1 + 2^-7: ties go to the even 1
        assert_eq!(bf16_from_f32(1.0 + 2f32.powi(-8)), 0x3f80);
        // 1 + 3·2^-8 is halfway between 1 + 2^-7 and 1 + 2^-6: ties go to the even 1 + 2^-6
        assert_eq!(bf16_from_f32(1.0 + 3.0 * 2f32.powi(-8)), 0x3f82);
        // Just past a tie rounds up
        assert_eq!(bf16_from_f32(f32::from_bits(0x3f80_8001)), 0x3f81);
        // Rounding up into the next exponent
        assert_eq!(bf16_from_f32(f32::from_bits(0x3fff_ffff)), 0x4000);
        // Overflow: f32::MAX rounds past the largest BF16
        assert_eq!(bf16_from_f32(f32::MAX), 0x7f80);
        assert_eq!(bf16_from_f32(f32::from_bits(0x7f7f_7fff)), 0x7f7f);
        assert_eq!(bf16_from_f32(-f32::MAX), 0xff80);
        // Infinities stay infinite; NaNs stay NaN, even with a payload in the low bits only
        assert_eq!(bf16_from_f32(f32::INFINITY), 0x7f80);
        assert_eq!(bf16_from_f32(f32::from_bits(0x7f80_0001)), 0x7fc0);
        assert_eq!(bf16_from_f32(f32::from_bits(0xff80_ffff)), 0xffc0);
        // Subnormals round like normals
        assert_eq!(bf16_from_f32(f32::from_bits(0x0000_8000)), 0x0000); // tie, to even 0
        assert_eq!(bf16_from_f32(f32::from_bits(0x0001_8000)), 0x0002); // tie, to even 2
        assert_eq!(bf16_from_f32(f32::from_bits(0x0000_8001)), 0x0001);
    }

    // ----- FP4 weights -----

    const MX: u64 = DOT_BF16_FLAGS_MXFP4;
    const NV: u64 = DOT_BF16_FLAGS_NVFP4;

    /// The E2M1 code of an exactly representable value.
    fn e2m1(v: f32) -> u8 {
        let c = (0u8..16).find(|&c| e2m1_units(c) as f32 / 2.0 == v && !(v == 0.0 && c != 0));
        c.unwrap_or_else(|| panic!("{v} is not an E2M1 value"))
    }

    /// Packs E2M1 values two per byte, low nibble first.
    fn pack(v: &[f32]) -> Vec<u8> {
        v.chunks(2).map(|p| e2m1(p[0]) | e2m1(*p.get(1).unwrap_or(&0.0)) << 4).collect()
    }

    /// E8M0 scale byte of 2^k.
    fn e8m0(k: i32) -> u8 {
        (k + 127) as u8
    }

    /// E4M3 code of an exactly representable scale.
    fn e4m3(v: f32) -> u8 {
        let t = &crate::DOT_F8_DECODE[crate::DOT_F8_E4M3 as usize];
        (0u8..=0x7e).find(|&c| t[c as usize] as f64 / 512.0 == v as f64).expect("E4M3 value")
    }

    /// A vector of `n` values with `vals` at its start, the rest 0.
    fn at_start<T: Copy + Default>(n: usize, vals: &[T]) -> Vec<T> {
        let mut v = vec![T::default(); n];
        v[..vals.len()].copy_from_slice(vals);
        v
    }

    fn fp4(flags: u64, x: &[f32], scales: &[u8], y: &[u16]) -> u32 {
        dot_bf16_fp4_reference(flags, &pack(x), scales, y, x.len(), 0, None)
    }

    // The basic tests of precompiles/dot_bf16/fp4.md, numbered as there.

    #[test]
    fn fp4_01_basic() {
        let x = at_start(32, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(fp4(MX, &x, &[e8m0(0)], &[ONE; 32]), 0x4120_0000);
    }

    #[test]
    fn fp4_02_scale_is_applied() {
        let x = at_start(32, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(fp4(MX, &x, &[e8m0(-3)], &[ONE; 32]), 1.25f32.to_bits());
    }

    #[test]
    fn fp4_03_cancellation_at_the_top() {
        let x = at_start(32, &[6.0, -6.0]);
        let y = at_start(32, &[pow2(100), pow2(100)]);
        assert_eq!(fp4(MX, &x, &[e8m0(127)], &y), 0);
    }

    #[test]
    fn fp4_04_smallest_product_underflows() {
        let x = at_start(32, &[0.5]);
        let y = at_start(32, &[pow2(-133)]);
        assert_eq!(fp4(MX, &x, &[e8m0(-127)], &y), 0);
    }

    #[test]
    fn fp4_05_overflow() {
        let x = at_start(32, &[6.0]);
        let y = at_start(32, &[MAX]);
        assert_eq!(fp4(MX, &x, &[e8m0(127)], &y), 0x7f80_0000);
    }

    #[test]
    fn fp4_06_07_one_scale_per_block_and_block_results() {
        let mut x = vec![0.0; 64];
        (x[0], x[32]) = (1.0, 1.0);
        let scales = [e8m0(10), e8m0(-20)];
        assert_eq!(fp4(MX, &x, &scales, &[ONE; 64]), 1024f32.to_bits());
        let mut out = [0u32; 2];
        let c = dot_bf16_fp4_reference(MX, &pack(&x), &scales, &[ONE; 64], 64, 32, Some(&mut out));
        assert_eq!(c, 1024f32.to_bits());
        assert_eq!(out, [1024f32.to_bits(), 2f32.powi(-20).to_bits()]);
    }

    #[test]
    fn fp4_08_nan_scale() {
        let x = at_start(32, &[1.0]);
        assert_eq!(fp4(MX, &x, &[0xff], &[ONE; 32]), DOT_BF16_NAN);
        let x = at_start(16, &[1.0]);
        assert_eq!(fp4(NV, &x, &[0x7f], &[ONE; 16]), DOT_BF16_NAN);
        assert_eq!(fp4(NV, &x, &[0xff], &[ONE; 16]), DOT_BF16_NAN);
    }

    #[test]
    fn fp4_09_bf16_specials() {
        let x = at_start(32, &[1.0]);
        assert_eq!(fp4(MX, &x, &[e8m0(0)], &at_start(32, &[INF])), DOT_BF16_NAN);
    }

    #[test]
    fn fp4_10_tie_across_blocks() {
        let mut x = vec![0.0; 64];
        (x[0], x[32]) = (1.0, 0.5);
        assert_eq!(fp4(MX, &x, &[e8m0(0), e8m0(-23)], &[ONE; 64]), 0x3f80_0000);
    }

    #[test]
    fn fp4_nvfp4_fractional_scales() {
        // 1·1.5 + 2·0.375 (two blocks of 16): 1.5 + 0.75 = 2.25
        let mut x = vec![0.0; 32];
        (x[0], x[16]) = (1.0, 2.0);
        assert_eq!(fp4(NV, &x, &[e4m3(1.5), e4m3(0.375)], &[ONE; 32]), 2.25f32.to_bits());
        // The largest E4M3 scale and E2M1 value: 6 · 448 · 2 = 5376
        let x = at_start(16, &[6.0]);
        assert_eq!(fp4(NV, &x, &[e4m3(448.0)], &at_start(16, &[bf(2.0)])), 5376f32.to_bits());
        // The smallest E4M3 subnormal scale, 2^-9: 0.5 · 2^-9 · 1 = 2^-10
        let x = at_start(16, &[0.5]);
        assert_eq!(fp4(NV, &x, &[0x01], &[ONE; 16]), 2f32.powi(-10).to_bits());
        // A negative scale flips the sign
        let x = at_start(16, &[1.0]);
        assert_eq!(fp4(NV, &x, &[e4m3(1.0) | 0x80], &[ONE; 16]), (-1f32).to_bits());
    }

    /// Exact oracle: every product times 2^60 is an integer for these ranges, so the sum is
    /// exact in i128; `as f32` rounds to nearest even, and the scaling is exact.
    fn fp4_oracle(flags: u64, x: &[u8], scales: &[u8], y: &[u16], n: usize) -> u32 {
        let mut sum: i128 = 0;
        for i in 0..n {
            let (nw, sw, ew) =
                fp4_weight_decode(flags, fp4_code(x, i), scales[i / dot_bf16_fp4_block(flags)])
                    .unwrap();
            let (nv, sv, ev) = bf16_decode(y[i]).unwrap();
            let p = ((sw as i128) * (sv as i128)) << (ew + ev + 60);
            sum += if nw != nv { -p } else { p };
        }
        if sum == 0 {
            return 0;
        }
        ((sum as f32) * 2f32.powi(-60)).to_bits()
    }

    #[test]
    fn fp4_11_random_against_exact_oracle() {
        let mut r = Rng(0x5151_7272_9393_b4b4);
        for flags in [MX, NV] {
            let fb = dot_bf16_fp4_block(flags);
            for _ in 0..5_000 {
                let n = 32 * (1 + (r.next() % 64) as usize);
                let x: Vec<u8> = (0..n / 2).map(|_| r.next() as u8).collect();
                let scales: Vec<u8> = (0..n / fb)
                    .map(|_| {
                        if flags == MX {
                            e8m0((r.next() % 21) as i32 - 10)
                        } else {
                            (r.next() % 0x7f) as u8 // any finite positive E4M3
                        }
                    })
                    .collect();
                let y: Vec<u16> = (0..n).map(|_| random_bf16(&mut r, -8, 8)).collect();
                let c = dot_bf16_fp4_reference(flags, &x, &scales, &y, n, 0, None);
                assert_eq!(c, fp4_oracle(flags, &x, &scales, &y, n), "flags {flags}, n = {n}");
            }
        }
    }

    #[test]
    fn fp4_12_order_of_blocks_does_not_matter() {
        let mut r = Rng(0x0f0f_1e1e_2d2d_3c3c);
        for _ in 0..500 {
            let blocks = 1 + (r.next() % 16) as usize;
            let n = 32 * blocks;
            let x: Vec<u8> = (0..n / 2).map(|_| r.next() as u8).collect();
            let scales: Vec<u8> = (0..blocks).map(|_| (r.next() % 255) as u8).collect();
            let y: Vec<u16> = (0..n).map(|_| random_bf16(&mut r, -126, 127)).collect();
            let c = dot_bf16_fp4_reference(MX, &x, &scales, &y, n, 0, None);
            // Reverse the order of the blocks
            let (mut x2, mut s2, mut y2) = (Vec::new(), Vec::new(), Vec::new());
            for b in (0..blocks).rev() {
                x2.extend_from_slice(&x[16 * b..16 * (b + 1)]);
                s2.push(scales[b]);
                y2.extend_from_slice(&y[32 * b..32 * (b + 1)]);
            }
            assert_eq!(dot_bf16_fp4_reference(MX, &x2, &s2, &y2, n, 0, None), c);
        }
    }

    #[test]
    fn fp4_13_every_code_and_scale_decodes_exactly() {
        for flags in [MX, NV] {
            for code in 0u8..16 {
                for scale in 0..=255u8 {
                    let d = fp4_weight_decode(flags, code, scale);
                    let nan = if flags == MX { scale == 0xff } else { scale & 0x7f == 0x7f };
                    assert_eq!(d.is_none(), nan, "flags {flags} scale {scale:#04x}");
                    let Some((neg, sig, exp)) = d else { continue };
                    let s = if flags == MX {
                        2f64.powi(scale as i32 - 127)
                    } else {
                        crate::DOT_F8_DECODE[crate::DOT_F8_E4M3 as usize][scale as usize] as f64
                            / 512.0
                    };
                    let expected = e2m1_units(code) as f64 / 2.0 * s;
                    let got = (if neg { -1.0 } else { 1.0 }) * sig as f64 * 2f64.powi(exp);
                    assert_eq!(got.abs(), expected.abs(), "flags {flags} {code} {scale:#04x}");
                    if expected != 0.0 {
                        assert_eq!(got, expected, "flags {flags} {code} {scale:#04x}");
                    }
                }
            }
        }
    }
}
