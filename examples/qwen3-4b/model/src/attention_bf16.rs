//! BF16 attention as PyTorch 2.13.0 CPU SDPA computes it on macOS arm64 (no oneDNN, no
//! SVE): the flash kernel with 512-key blocks, `cpublas::gemm`'s fallback with four
//! interleaved partial sums for both products, NEON `fexp_u20` with libsystem `expf`
//! for each row's tail, the softmax weights rounded to BF16, and the output scaled by
//! the reciprocal of the row sum and rounded to BF16.

use crate::decoder::bf16_value;

/// The kernel's key block.
const KV_SPLIT: usize = 512;

/// One query row of SDPA: `out` gets the attention of `q` at position `t` over the keys
/// and values of positions `0..=t`, row `s` of each at `s * stride` in `k` and `v`.
/// Positions below `prefill` are one causal call over `prefill` keys; each later one
/// is a cached decode step over `t + 1`.
pub(crate) fn attend(
    out: &mut [f32],
    q: &[f32],
    k: &[f32],
    v: &[f32],
    stride: usize,
    t: usize,
    prefill: usize,
) {
    let hd = q.len();
    let len = if t < prefill { prefill } else { t + 1 };
    let scale = (1.0 / libm::sqrt(hd as f64)) as f32;
    let (mut max, mut sum) = (f32::NEG_INFINITY, 0.0f32);
    let mut acc = vec![0.0f32; hd];
    let mut e = vec![0.0f32; len.min(KV_SPLIT)];
    for n in (0..=t).step_by(KV_SPLIT) {
        let e = &mut e[..KV_SPLIT.min(len - n)];
        for (s, x) in e.iter_mut().enumerate() {
            let key = &k[(n + s) * stride..][..hd];
            *x = match n + s <= t {
                true => sum4(hd, |i| bf16_value(q[i]) * bf16_value(key[i])) * scale,
                false => f32::NEG_INFINITY,
            };
        }
        let new = e.iter().fold(max, |m, &x| m.max(x));
        let (v8, v4) = (e.len() / 8 * 8, e.len() / 4 * 4);
        let (mut s0, mut s1, mut tail) = ([0.0f32; 4], [0.0f32; 4], [0.0f32; 4]);
        for (i, x) in e[..v4].iter_mut().enumerate() {
            *x = fexp_u20(*x - new);
            match (i < v8, i % 8 < 4) {
                (true, true) => s0[i % 4] += *x,
                (true, false) => s1[i % 4] += *x,
                (false, _) => tail[i % 4] += *x,
            }
        }
        let lane = |l: usize| s0[l] + tail[l] + s1[l];
        let mut block = (lane(0) + lane(2)) + (lane(1) + lane(3));
        for x in &mut e[v4..] {
            *x = expf(*x - new);
            block += *x;
        }
        let rescale = expf(max - new);
        // The pinned arm64 kernel fuses the carry from the previous key block.
        sum = libm::fmaf(rescale, sum, block);
        max = new;
        for (i, a) in acc.iter_mut().enumerate() {
            let dot = sum4(e.len(), |s| bf16_value(v[(n + s) * stride + i]) * bf16_value(e[s]));
            *a = if n == 0 { dot } else { *a * rescale + dot };
        }
    }
    let inv = 1.0 / sum;
    for (o, a) in out.iter_mut().zip(acc) {
        *o = bf16_value(a * inv);
    }
}

/// `cpublas`' `sum`: four partial sums, the remainder into the first, then folded in order.
fn sum4(n: usize, f: impl Fn(usize) -> f32) -> f32 {
    let mut p = [0.0f32; 4];
    for i in 0..n / 4 * 4 {
        p[i % 4] += f(i);
    }
    for i in n / 4 * 4..n {
        p[0] += f(i);
    }
    p[0] + p[1] + p[2] + p[3]
}

/// NEON `Vectorized<float>::fexp_u20` for `x <= 0`.
fn fexp_u20(x: f32) -> f32 {
    if x < f32::from_bits(0xc2af5dc1) {
        return 0.0;
    }
    let n = libm::roundf(x * f32::from_bits(0x3fb8aa3b));
    let r = libm::fmaf(-n, f32::from_bits(0x3f317218), x);
    let q = libm::fmaf(r, f32::from_bits(0x3e2ac976), f32::from_bits(0x3f00be9a));
    let p = libm::fmaf(q, r * r, 1.0 + r);
    f32::from_bits(p.to_bits().wrapping_add((n as i32 as u32) << 23))
}

/// macOS libsystem `expf`, bit for bit (checked against it on every `f32`).
fn expf(x: f32) -> f32 {
    if (x.to_bits() >> 23) & 0xff >= 0x86 {
        return if x < 0.0 { 0.0 } else { x * f32::MAX };
    }
    let (k, shift) = (f64::from_bits(0x40671547652b82fe), f64::from_bits(0x4338000000000000));
    let x = x as f64;
    let t = libm::fma(k, x, shift);
    let r = libm::fma(k, x, shift - t);
    let i = t.to_bits();
    let s = f64::from_bits(TABLE[(i & 0x7f) as usize].wrapping_add(i << 45));
    let p =
        libm::fma(f64::from_bits(0x3eeebfbdff30d656), r, f64::from_bits(0x3f762e4453e10dae)) * r;
    libm::fma(p, s, s) as f32
}

/// `2^(j/128)` less `j << 45`, as libsystem `expf` stores it.
const TABLE: [u64; 128] = [
    0x3ff0000000000000,
    0x3feff63da9fb3335,
    0x3fefec9a3e778061,
    0x3fefe315e86e7f85,
    0x3fefd9b0d3158574,
    0x3fefd06b29ddf6de,
    0x3fefc74518759bc8,
    0x3fefbe3ecac6f383,
    0x3fefb5586cf9890f,
    0x3fefac922b7247f7,
    0x3fefa3ec32d3d1a2,
    0x3fef9b66affed31b,
    0x3fef9301d0125b51,
    0x3fef8abdc06c31cc,
    0x3fef829aaea92de0,
    0x3fef7a98c8a58e51,
    0x3fef72b83c7d517b,
    0x3fef6af9388c8dea,
    0x3fef635beb6fcb75,
    0x3fef5be084045cd4,
    0x3fef54873168b9aa,
    0x3fef4d5022fcd91d,
    0x3fef463b88628cd6,
    0x3fef3f49917ddc96,
    0x3fef387a6e756238,
    0x3fef31ce4fb2a63f,
    0x3fef2b4565e27cdd,
    0x3fef24dfe1f56381,
    0x3fef1e9df51fdee1,
    0x3fef187fd0dad990,
    0x3fef1285a6e4030b,
    0x3fef0cafa93e2f56,
    0x3fef06fe0a31b715,
    0x3fef0170fc4cd831,
    0x3feefc08b26416ff,
    0x3feef6c55f929ff1,
    0x3feef1a7373aa9cb,
    0x3feeecae6d05d866,
    0x3feee7db34e59ff7,
    0x3feee32dc313a8e5,
    0x3feedea64c123422,
    0x3feeda4504ac801c,
    0x3feed60a21f72e2a,
    0x3feed1f5d950a897,
    0x3feece086061892d,
    0x3feeca41ed1d0057,
    0x3feec6a2b5c13cd0,
    0x3feec32af0d7d3de,
    0x3feebfdad5362a27,
    0x3feebcb299fddd0d,
    0x3feeb9b2769d2ca7,
    0x3feeb6daa2cf6642,
    0x3feeb42b569d4f82,
    0x3feeb1a4ca5d920f,
    0x3feeaf4736b527da,
    0x3feead12d497c7fd,
    0x3feeab07dd485429,
    0x3feea9268a5946b7,
    0x3feea76f15ad2148,
    0x3feea5e1b976dc09,
    0x3feea47eb03a5585,
    0x3feea34634ccc320,
    0x3feea23882552225,
    0x3feea155d44ca973,
    0x3feea09e667f3bcd,
    0x3feea012750bdabf,
    0x3fee9fb23c651a2f,
    0x3fee9f7df9519484,
    0x3fee9f75e8ec5f74,
    0x3fee9f9a48a58174,
    0x3fee9feb564267c9,
    0x3feea0694fde5d3f,
    0x3feea11473eb0187,
    0x3feea1ed0130c132,
    0x3feea2f336cf4e62,
    0x3feea427543e1a12,
    0x3feea589994cce13,
    0x3feea71a4623c7ad,
    0x3feea8d99b4492ed,
    0x3feeaac7d98a6699,
    0x3feeace5422aa0db,
    0x3feeaf3216b5448c,
    0x3feeb1ae99157736,
    0x3feeb45b0b91ffc6,
    0x3feeb737b0cdc5e5,
    0x3feeba44cbc8520f,
    0x3feebd829fde4e50,
    0x3feec0f170ca07ba,
    0x3feec49182a3f090,
    0x3feec86319e32323,
    0x3feecc667b5de565,
    0x3feed09bec4a2d33,
    0x3feed503b23e255d,
    0x3feed99e1330b358,
    0x3feede6b5579fdbf,
    0x3feee36bbfd3f37a,
    0x3feee89f995ad3ad,
    0x3feeee07298db666,
    0x3feef3a2b84f15fb,
    0x3feef9728de5593a,
    0x3feeff76f2fb5e47,
    0x3fef05b030a1064a,
    0x3fef0c1e904bc1d2,
    0x3fef12c25bd71e09,
    0x3fef199bdd85529c,
    0x3fef20ab5fffd07a,
    0x3fef27f12e57d14b,
    0x3fef2f6d9406e7b5,
    0x3fef3720dcef9069,
    0x3fef3f0b555dc3fa,
    0x3fef472d4a07897c,
    0x3fef4f87080d89f2,
    0x3fef5818dcfba487,
    0x3fef60e316c98398,
    0x3fef69e603db3285,
    0x3fef7321f301b460,
    0x3fef7c97337b9b5f,
    0x3fef864614f5a129,
    0x3fef902ee78b3ff6,
    0x3fef9a51fbc74c83,
    0x3fefa4afa2a490da,
    0x3fefaf482d8e67f1,
    0x3fefba1bee615a27,
    0x3fefc52b376bba97,
    0x3fefd0765b6e4540,
    0x3fefdbfdad9cbe14,
    0x3fefe7c1819e90d8,
    0x3feff3c22b8f71f1,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(n: usize, a: usize, b: usize, c: f32, d: f32) -> Vec<f32> {
        (0..n).map(|i| bf16_value(((i * a % b) as f32 - c) / d)).collect()
    }

    fn digest(values: &[f32]) -> u64 {
        values.iter().fold(0xcbf29ce484222325, |a, y| {
            (a ^ u64::from(y.to_bits() >> 16)).wrapping_mul(0x100000001b3)
        })
    }

    #[test]
    fn prefill_and_decode_at_key_block_boundaries() {
        let mut fixtures = include_str!("../tests/fixtures/attention_boundaries.txt").lines();
        for n in [511usize, 512, 513, 1023, 1024, 1025] {
            let line = fixtures.next().expect("missing Torch fixture");
            let fields: Vec<_> = line.split_whitespace().collect();
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0].parse::<usize>().unwrap(), n);
            let q = ramp(n * 4 * 128, 37, 211, 105.0, 61.0);
            let k = ramp(n * 128, 53, 197, 98.0, 29.0);
            let v = ramp(n * 128, 29, 101, 50.0, 7.0);
            for (first, prefill, expected) in [(n - 2, n, fields[1]), (n - 1, 1, fields[2])] {
                let mut rows = Vec::new();
                for t in first..n {
                    for head in 0..4 {
                        let start = (4 * t + head) * 128;
                        let mut out = [0.0; 128];
                        attend(&mut out, &q[start..start + 128], &k, &v, 128, t, prefill);
                        rows.extend_from_slice(&out);
                    }
                }
                assert_eq!(
                    digest(&rows),
                    u64::from_str_radix(expected, 16).unwrap(),
                    "context {n}, prefill {prefill}"
                );
            }
        }
        assert!(fixtures.next().is_none());
    }

    #[test]
    fn cached_decode_rescales_the_previous_softmax_sum_with_one_rounding() {
        let mut keys = [4.75; 518];
        for pair in keys[..512].chunks_exact_mut(2) {
            pair.copy_from_slice(&[0.0, -0.125]);
        }
        let mut values = [176.0; 518];
        values[..512].fill(1.0);
        let mut out = [0.0];
        attend(&mut out, &[1.0], &keys, &values, 1, 517, 1);
        assert_eq!(out, [104.5]); // Pinned Torch CPU SDPA returns this BF16 value.
    }

    #[test]
    fn matches_torch_arm64_prefill_and_cached_decode() {
        // FNV-1a over the BF16 outputs of PyTorch 2.13.0 CPU SDPA on macOS arm64, called
        // as HF Qwen3 does: one causal prefill of `p` rows, then one decode step per row.
        // The second case crosses the 512-key block.
        for (n, p, h, kvh, expected) in
            [(20, 13, 4, 2, 0x356b52f4f9fda748u64), (515, 513, 2, 1, 0x22f5c7f91649aa0)]
        {
            let (hd, qd, kvd) = (128, h * 128, kvh * 128);
            let q = ramp(n * qd, 37, 211, 105.0, 61.0);
            let k = ramp(n * kvd, 53, 197, 98.0, 29.0);
            let v = ramp(n * kvd, 29, 101, 50.0, 7.0);
            let mut out = vec![0.0f32; n * qd];
            for t in 0..n {
                for j in 0..h {
                    let kv = j / (h / kvh) * hd;
                    attend(
                        &mut out[t * qd + j * hd..][..hd],
                        &q[t * qd + j * hd..][..hd],
                        &k[kv..],
                        &v[kv..],
                        kvd,
                        t,
                        p,
                    );
                }
            }
            assert_eq!(digest(&out), expected);
        }
    }
}
