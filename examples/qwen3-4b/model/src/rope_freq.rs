//! Scalar positive-base path of SLEEF powf_u10 for Qwen's rotary frequencies.
//! Adapted from SLEEF 3.9.0, src/libm/sleefsimdsp.c (logkf and expkf) and
//! src/common/df.h (compensated FP32 operations, FMA configuration).
//! https://github.com/shibatch/sleef/tree/3.9.0/src
//! Copyright Naoki Shibata and contributors 2010 - 2025.
//! Distributed under the Boost Software License, Version 1.0 (see ../LICENSE-SLEEF.txt).

use libm::fmaf;

// hi holds the FP32 result of an operation; lo carries its rounding error.
#[derive(Clone, Copy)]
struct FloatPair {
    hi: f32,
    lo: f32,
}

impl FloatPair {
    fn sum(a: f32, b: f32) -> Self {
        let hi = a + b;
        let v = hi - a;
        Self { hi, lo: (a - (hi - v)) + (b - v) }
    }

    fn add(self, other: Self) -> Self {
        let sum = Self::sum(self.hi, other.hi);
        Self { hi: sum.hi, lo: sum.lo + (self.lo + other.lo) }
    }

    // SLEEF's short addition when the leading term dominates.
    fn add_small(self, other: Self) -> Self {
        let hi = self.hi + other.hi;
        Self { hi, lo: (((self.hi - hi) + other.hi) + self.lo) + other.lo }
    }

    fn times(self, b: f32) -> Self {
        let hi = self.hi * b;
        Self { hi, lo: fmaf(self.lo, b, fmaf(self.hi, b, -hi)) }
    }

    fn product(self, b: Self) -> Self {
        let hi = self.hi * b.hi;
        let error = fmaf(self.lo, b.hi, fmaf(self.hi, b.hi, -hi));
        Self { hi, lo: fmaf(self.hi, b.lo, error) }
    }

    fn square(self) -> Self {
        let hi = self.hi * self.hi;
        Self { hi, lo: fmaf(self.hi + self.hi, self.lo, fmaf(self.hi, self.hi, -hi)) }
    }

    fn quotient(self, b: Self) -> Self {
        let inv = 1.0 / b.hi;
        let hi = self.hi * inv;
        let error = fmaf(self.hi, inv, -hi);
        let residual = fmaf(-b.lo, inv, fmaf(-b.hi, inv, 1.0));
        Self { hi, lo: fmaf(hi, residual, fmaf(self.lo, inv, error)) }
    }
}

fn log_kernel(theta: f32) -> FloatPair {
    // Reduce theta to m * 2^e with m near 1. Decomposing first also
    // keeps the multiplication used to select the binade in the normal range.
    let (fraction, exponent) = libm::frexpf(theta);
    let shift = libm::ilogbf(fraction * (1.0 / 0.75));
    let m = libm::scalbnf(fraction, -shift);
    let e = (exponent + shift) as f32;
    let x = FloatPair::sum(-1.0, m).quotient(FloatPair::sum(1.0, m));
    let x2 = x.square();
    let t = fmaf(0.24032035, x2.hi, 0.28511268);
    let t = fmaf(t, x2.hi, 0.400008);
    let c = FloatPair { hi: 0.6666666, lo: 3.6918386e-9 };
    let ln2 = FloatPair { hi: core::f32::consts::LN_2, lo: -1.9046543e-9 };
    let leading = ln2.times(e).add_small(FloatPair { hi: 2.0 * x.hi, lo: 2.0 * x.lo });
    leading.add_small(x2.product(x).product(x2.times(t).add(c)))
}

fn exp_kernel(d: FloatPair) -> f32 {
    let q = libm::rintf((d.hi + d.lo) * core::f32::consts::LOG2_E);
    let s = d
        .add(FloatPair { hi: q * -0.69314575, lo: 0.0 })
        .add(FloatPair { hi: q * -1.4286068e-6, lo: 0.0 });
    let hi = s.hi + s.lo;
    let s = FloatPair { hi, lo: (s.hi - hi) + s.lo };

    let u = fmaf(0.0013632465, s.hi, 0.008365969);
    let u = fmaf(u, s.hi, 0.041671082);
    let u = fmaf(u, s.hi, 0.16666552);
    let u = fmaf(u, s.hi, 0.49999985);
    let t = s.add_small(s.square().times(u));
    let t = FloatPair { hi: 1.0, lo: 0.0 }.add_small(t);
    libm::scalbnf(t.hi + t.lo, q as i32)
}

/// Torch rounds the positive power to FP32 before taking its FP32 reciprocal.
pub(super) fn inverse_frequencies(theta: f32, head_dim: usize) -> Vec<f32> {
    assert!(theta.is_finite() && theta > 0.0 && head_dim > 0);
    let log_theta = log_kernel(theta);
    (0..head_dim)
        .step_by(2)
        .map(|k| {
            let exponent = k as f32 / head_dim as f32;
            1.0 / exp_kernel(log_theta.times(exponent))
        })
        .collect()
}
