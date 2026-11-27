//! Scalar RoPE trigonometry for the pinned Qwen3-4B-Instruct-2507 CPU profile.
//! The input is the FP32 product of a position (0..40960) and one of the
//! 64 inverse frequencies. Round each returned FP32 value to BF16 before
//! rotating the query or key. Parity with arm64 Torch is at that BF16 boundary.

// Polynomial coefficients from SLEEF's xsinf_u1 and xcosf_u1:
// https://github.com/shibatch/sleef/blob/7623d6cfa2712462880fa63a4d0f0b5f775d1a83/src/libm/sleefsimdsp.c
// Copyright Naoki Shibata and contributors 2010 - 2025.
// This scalar adaptation uses binary64 for range reduction and evaluation.
// Distributed under the Boost Software License, Version 1.0 (see ../LICENSE-SLEEF.txt).

use core::f64::consts::PI;

/// Evaluate the odd sine polynomial on [-pi/2, pi/2].
fn reduced_sin(r: f64) -> f32 {
    let t = r * r;
    let q = ((2.6083159809786594e-6 * t - 0.00019810690719168633) * t + 0.00833307858556509) * t
        - 0.16666659712791443;
    (r * (1.0 + t * q)) as f32
}

pub(super) fn cos_sin(angle: f32) -> (f32, f32) {
    let x = f64::from(angle);
    let half_turns = x / PI;
    // Reduce cosine around (j + 0.5) * pi and sine around k * pi.
    // Each pi shift flips the sign.
    let j = half_turns as u32;
    let k = (half_turns + 0.5) as u32;
    let c = reduced_sin(x - (f64::from(j) + 0.5) * PI);
    let s = reduced_sin(x - f64::from(k) * PI);
    (if j % 2 == 0 { -c } else { c }, if k % 2 == 0 { s } else { -s })
}
