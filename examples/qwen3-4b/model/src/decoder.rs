pub fn bf16_value(x: f32) -> f32 {
    let bits = x.to_bits();
    if x.is_nan() {
        return f32::from_bits((bits & 0xffff0000) | 0x00400000);
    }
    f32::from_bits(bits.wrapping_add(0x7fff + ((bits >> 16) & 1)) & 0xffff0000)
}
pub(crate) fn bf16_rmsnorm(out: &mut [f32], x: &[f32], weight: &[f32], eps: f32) {
    // arm64 PyTorch SumKernel uses four interleaved four-lane sums,
    // carrying each group of 16 iterations. Both Qwen widths fit two levels.
    assert!(matches!(x.len(), 128 | 2560));
    let mut totals = [0.0f32; 16];
    for chunk in x.chunks(256) {
        let mut partial = [0.0f32; 16];
        for row in chunk.chunks_exact(16) {
            for (acc, &value) in partial.iter_mut().zip(row) {
                let v = bf16_value(value);
                *acc += v * v;
            }
        }
        for (acc, part) in totals.iter_mut().zip(partial) {
            *acc += part;
        }
    }
    let sum: f32 = (0..4).map(|i| totals[i] + totals[i + 4] + totals[i + 8] + totals[i + 12]).sum();
    let inv = 1.0 / libm::sqrtf(sum / x.len() as f32 + eps);
    for ((o, &v), &w) in out.iter_mut().zip(x).zip(weight) {
        *o = bf16_value(bf16_value(bf16_value(v) * inv) * w);
    }
}
// Queries and keys share these angles across every head at the same position.
fn rope_angles(pos: usize, inv_freq: &[f32], angles: &mut [(f32, f32)]) {
    for (&frequency, pair) in inv_freq.iter().zip(angles) {
        let (cos, sin) = crate::rope_trig::cos_sin(pos as f32 * frequency);
        *pair = (bf16_value(cos), bf16_value(sin));
    }
}
/// HF rotate_half rounds each product and each sum in the BF16 serving profile.
fn rope(v: &mut [f32], angles: &[(f32, f32)]) {
    let half = v.len() / 2;
    for (i, &(c, s)) in angles.iter().enumerate() {
        let (x1, x2) = (v[i], v[i + half]);
        v[i] = bf16_value(bf16_value(x1 * c) - bf16_value(x2 * s));
        v[i + half] = bf16_value(bf16_value(x2 * c) + bf16_value(x1 * s));
    }
}
fn residual(x: &mut [f32], o: &[f32]) {
    for (a, b) in x.iter_mut().zip(o) {
        *a = bf16_value(*a + b);
    }
}
fn silu_bf16(x: f32) -> f32 {
    static TABLE: &[u8; 2 * 65536] = include_bytes!(concat!(env!("OUT_DIR"), "/silu-bf16.bin"));
    let bits = x.to_bits();
    if bits & 0xffff != 0 || !x.is_finite() {
        return bf16_value(x / (1.0 + libm::expf(-x)));
    }
    let offset = ((bits >> 16) as usize) * 2;
    let y = u16::from_le_bytes([TABLE[offset], TABLE[offset + 1]]);
    f32::from_bits(u32::from(y) << 16)
}
pub const BATCH_GROUPS: [&[usize]; 4] = [&[0, 1, 2], &[3], &[4, 5], &[6]];

/// Fills `outs[k]` (`n_tok x rows`, row-major) with the product of matrix
/// `BATCH_GROUPS[group][k]` and every row of `x` (`n_tok x cols`); called once per group,
/// in order.
pub type ClaimedBatch<'a> = &'a mut dyn FnMut(usize, &[f32], &mut [&mut [f32]]);

pub(crate) fn run_layer_batched(
    mut x: Vec<f32>,
    norm: &[Vec<f32>; 4],
    prefill: usize,
    theta: f32,
    hd: usize,
    claimed: ClaimedBatch,
) -> Vec<f32> {
    let [input_ln, q_norm, k_norm, post_ln] = norm;
    let (dim, hidden, heads, kv_heads) = (2560, 9728, 32, 8);
    let n = x.len() / dim;
    let (qd, kvd) = (heads * hd, kv_heads * hd);
    let inv_freq = crate::rope_freq::inverse_frequencies(theta, hd);
    let mut angles = vec![(0.0, 0.0); inv_freq.len()];
    let mut xn = vec![0.0f32; n * dim];
    let mut q = vec![0.0f32; n * qd];
    let mut k = vec![0.0f32; n * kvd];
    let mut v = vec![0.0f32; n * kvd];
    let mut tmp = vec![0.0f32; hd];
    for t in 0..n {
        bf16_rmsnorm(&mut xn[t * dim..][..dim], &x[t * dim..][..dim], input_ln, 1e-6);
    }
    claimed(0, &xn, &mut [&mut q, &mut k, &mut v]);
    for t in 0..n {
        rope_angles(t, &inv_freq, &mut angles);
        for (heads, weight, buf) in
            [(heads, q_norm, &mut q[t * qd..][..qd]), (kv_heads, k_norm, &mut k[t * kvd..][..kvd])]
        {
            for j in 0..heads {
                let head = &mut buf[j * hd..][..hd];
                tmp.copy_from_slice(head);
                bf16_rmsnorm(head, &tmp, weight, 1e-6);
                rope(head, &angles);
            }
        }
    }

    let group = heads / kv_heads;
    let mut attn = vec![0.0f32; n * qd];
    for t in 0..n {
        for j in 0..heads {
            let kv = (j / group) * hd;
            let (out, qh) = (&mut attn[t * qd + j * hd..][..hd], &q[t * qd + j * hd..][..hd]);
            crate::attention_bf16::attend(out, qh, &k[kv..], &v[kv..], kvd, t, prefill);
        }
    }
    drop((q, k, v));

    let mut o = vec![0.0f32; n * dim];
    claimed(1, &attn, &mut [&mut o]);
    drop(attn);
    for t in 0..n {
        let xt = &mut x[t * dim..][..dim];
        residual(xt, &o[t * dim..][..dim]);
        bf16_rmsnorm(&mut xn[t * dim..][..dim], xt, post_ln, 1e-6);
    }
    let mut g = vec![0.0f32; n * hidden];
    let mut u = vec![0.0f32; n * hidden];
    claimed(2, &xn, &mut [&mut g, &mut u]);
    for (gate, up) in g.iter_mut().zip(&u) {
        *gate = bf16_value(silu_bf16(*gate) * up);
    }
    drop(u);
    claimed(3, &g, &mut [&mut o]);
    residual(&mut x, &o);
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn rotary_matches_reference_across_supported_context() {
        // Reference from Qwen3RotaryEmbedding on the pinned arm64 PyTorch build.
        // Hash the FP32 frequencies, then independently check every BF16 output bit
        // via a digest in position-major order, with cosine before sine.
        let frequencies = crate::rope_freq::inverse_frequencies(5_000_000.0, 128);
        let mut hash = Sha256::new();
        for frequency in &frequencies {
            hash.update(frequency.to_le_bytes());
        }
        assert_eq!(
            format!("{:x}", hash.finalize_reset()),
            "29fc897bbb1fda6b314c1bcffa10e39b067913f538e4f07ed7cbd223b84c4cbf"
        );
        let mut angles = vec![(0.0, 0.0); frequencies.len()];
        for position in 0..40_960 {
            rope_angles(position, &frequencies, &mut angles);
            for &(cos, sin) in &angles {
                for v in [cos, sin] {
                    hash.update(((v.to_bits() >> 16) as u16).to_le_bytes());
                }
            }
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            "98352c6b6adc6275ba179eb4b092dd15d7e2dd4a657e2cd14df3fed57ef20db0"
        );
    }
}
