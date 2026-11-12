//! The transformer forward pass, following llama2.c's `runq.c`.

use alloc::vec;
use alloc::vec::Vec;

use crate::model::{Model, QMatrixView};

/// An activation vector quantized to int8 in groups, like a matrix row.
struct QVec {
    q: Vec<i8>,
    scales: Vec<f32>,
}

impl QVec {
    fn new(n: usize) -> Self {
        QVec { q: vec![0; n], scales: vec![0.0; n] }
    }

    /// Quantizes `x` in groups of `gs` (llama2.c's `quantize`).
    fn quantize(&mut self, x: &[f32], gs: usize) {
        for (g, group) in x.chunks_exact(gs).enumerate() {
            let wmax = group.iter().fold(0.0f32, |m, &v| m.max(libm::fabsf(v)));
            let scale = wmax / 127.0;
            self.scales[g] = scale;
            for (i, &v) in group.iter().enumerate() {
                let qv = if scale == 0.0 { 0.0 } else { libm::roundf(v / scale) };
                self.q[g * gs + i] = qv as i8;
            }
        }
    }
}

/// `out = W x`, with integer dot products per group (llama2.c's `matmul`).
fn matmul(out: &mut [f32], x: &QVec, w: &QMatrixView) {
    let (n, gs) = (w.cols, w.gs);
    for (i, o) in out.iter_mut().enumerate().take(w.rows) {
        let row = &w.q[i * n..(i + 1) * n];
        let row_scales = &w.scales[i * n / gs..(i + 1) * n / gs];
        let mut val = 0.0f32;
        for (g, (wg, xg)) in row.chunks_exact(gs).zip(x.q.chunks_exact(gs)).enumerate() {
            let mut ival: i32 = 0;
            for (&a, &b) in wg.iter().zip(xg) {
                ival += a as i32 * b as i32;
            }
            val += ival as f32 * row_scales[g] * x.scales[g];
        }
        *o = val;
    }
}

fn rmsnorm(out: &mut [f32], x: &[f32], weight: &[f32]) {
    let mut ss = 0.0f32;
    for &v in x {
        ss += v * v;
    }
    ss /= x.len() as f32;
    ss += 1e-5;
    ss = 1.0 / libm::sqrtf(ss);
    for ((o, &v), &w) in out.iter_mut().zip(x).zip(weight) {
        *o = w * (ss * v);
    }
}

/// In-place softmax.
pub fn softmax(x: &mut [f32]) {
    let max = x.iter().fold(x[0], |m, &v| m.max(v));
    let mut sum = 0.0f32;
    for v in x.iter_mut() {
        *v = libm::expf(*v - max);
        sum += *v;
    }
    for v in x.iter_mut() {
        *v /= sum;
    }
}

/// Activations and the KV cache for one sequence.
pub struct RunState {
    x: Vec<f32>,
    xb: Vec<f32>,
    xb2: Vec<f32>,
    hb: Vec<f32>,
    hb2: Vec<f32>,
    xq: QVec,
    hq: QVec,
    q: Vec<f32>,
    k: Vec<f32>,
    v: Vec<f32>,
    att: Vec<f32>,
    pub logits: Vec<f32>,
    key_cache: Vec<f32>,
    value_cache: Vec<f32>,
    /// RoPE frequency for each pair of dimensions within a head.
    rope_freq: Vec<f32>,
    rope_cos: Vec<f32>,
    rope_sin: Vec<f32>,
}

impl RunState {
    pub fn new(model: &Model) -> Self {
        let c = &model.config;
        let (dim, kv_dim, hidden, hs) = (c.dim, c.kv_dim(), c.hidden_dim, c.head_size());
        let rope_freq =
            (0..hs).step_by(2).map(|hd| 1.0 / libm::powf(10000.0, hd as f32 / hs as f32)).collect();
        RunState {
            x: vec![0.0; dim],
            xb: vec![0.0; dim],
            xb2: vec![0.0; dim],
            hb: vec![0.0; hidden],
            hb2: vec![0.0; hidden],
            xq: QVec::new(dim),
            hq: QVec::new(hidden),
            q: vec![0.0; dim],
            k: vec![0.0; kv_dim],
            v: vec![0.0; kv_dim],
            att: vec![0.0; c.n_heads * c.seq_len],
            logits: vec![0.0; c.vocab_size],
            key_cache: vec![0.0; c.n_layers * c.seq_len * kv_dim],
            value_cache: vec![0.0; c.n_layers * c.seq_len * kv_dim],
            rope_freq,
            rope_cos: vec![0.0; hs / 2],
            rope_sin: vec![0.0; hs / 2],
        }
    }
}

/// Runs the model on `token` at position `pos` and leaves the next-token
/// logits in `s.logits`.
pub fn forward(model: &Model, s: &mut RunState, token: u32, pos: usize) {
    let c = &model.config;
    let (dim, kv_dim, hidden, hs) = (c.dim, c.kv_dim(), c.hidden_dim, c.head_size());
    let kv_mul = c.n_heads / c.n_kv_heads;
    let (gs_dim, gs_hidden) = (model.wq.gs, model.w2.gs);

    // Token embedding, dequantized.
    let e = &model.embedding;
    let t = token as usize;
    for i in 0..dim {
        s.x[i] = e.q[t * dim + i] as f32 * e.scales[(t * dim + i) / e.gs];
    }

    // RoPE rotation for this position; the same for every layer and head.
    for (j, &freq) in s.rope_freq.iter().enumerate() {
        let val = pos as f32 * freq;
        s.rope_cos[j] = libm::cosf(val);
        s.rope_sin[j] = libm::sinf(val);
    }

    let inv_sqrt_hs = 1.0 / libm::sqrtf(hs as f32);
    for l in 0..c.n_layers {
        // Attention.
        rmsnorm(&mut s.xb, &s.x, &model.rms_att[l * dim..(l + 1) * dim]);
        s.xq.quantize(&s.xb, gs_dim);
        matmul(&mut s.q, &s.xq, &model.wq.rows(l * dim, dim));
        matmul(&mut s.k, &s.xq, &model.wk.rows(l * kv_dim, kv_dim));
        matmul(&mut s.v, &s.xq, &model.wv.rows(l * kv_dim, kv_dim));

        for i in (0..dim).step_by(2) {
            let j = (i % hs) / 2;
            let (fcr, fci) = (s.rope_cos[j], s.rope_sin[j]);
            let (v0, v1) = (s.q[i], s.q[i + 1]);
            s.q[i] = v0 * fcr - v1 * fci;
            s.q[i + 1] = v0 * fci + v1 * fcr;
            if i < kv_dim {
                let (v0, v1) = (s.k[i], s.k[i + 1]);
                s.k[i] = v0 * fcr - v1 * fci;
                s.k[i + 1] = v0 * fci + v1 * fcr;
            }
        }

        let loff = l * c.seq_len * kv_dim;
        s.key_cache[loff + pos * kv_dim..loff + (pos + 1) * kv_dim].copy_from_slice(&s.k);
        s.value_cache[loff + pos * kv_dim..loff + (pos + 1) * kv_dim].copy_from_slice(&s.v);

        for h in 0..c.n_heads {
            let q = &s.q[h * hs..(h + 1) * hs];
            let att = &mut s.att[h * c.seq_len..h * c.seq_len + pos + 1];
            let kv_off = (h / kv_mul) * hs;
            for (t, a) in att.iter_mut().enumerate() {
                let k = &s.key_cache[loff + t * kv_dim + kv_off..][..hs];
                let mut score = 0.0f32;
                for (&qi, &ki) in q.iter().zip(k) {
                    score += qi * ki;
                }
                *a = score * inv_sqrt_hs;
            }
            softmax(att);
            let xb = &mut s.xb[h * hs..(h + 1) * hs];
            xb.fill(0.0);
            for (t, &a) in att.iter().enumerate() {
                let v = &s.value_cache[loff + t * kv_dim + kv_off..][..hs];
                for (o, &vi) in xb.iter_mut().zip(v) {
                    *o += a * vi;
                }
            }
        }

        s.xq.quantize(&s.xb, gs_dim);
        matmul(&mut s.xb2, &s.xq, &model.wo.rows(l * dim, dim));
        for (x, &d) in s.x.iter_mut().zip(&s.xb2) {
            *x += d;
        }

        // Feed-forward: w2(silu(w1 x) * w3 x).
        rmsnorm(&mut s.xb, &s.x, &model.rms_ffn[l * dim..(l + 1) * dim]);
        s.xq.quantize(&s.xb, gs_dim);
        matmul(&mut s.hb, &s.xq, &model.w1.rows(l * hidden, hidden));
        matmul(&mut s.hb2, &s.xq, &model.w3.rows(l * hidden, hidden));
        for (h, &h2) in s.hb.iter_mut().zip(&s.hb2) {
            let mut val = *h;
            val *= 1.0 / (1.0 + libm::expf(-val));
            *h = val * h2;
        }
        s.hq.quantize(&s.hb, gs_hidden);
        matmul(&mut s.xb, &s.hq, &model.w2.rows(l * dim, dim));
        for (x, &d) in s.x.iter_mut().zip(&s.xb) {
            *x += d;
        }
    }

    let x = s.x.clone();
    rmsnorm(&mut s.x, &x, &model.rms_final);
    s.xq.quantize(&s.x, gs_dim);
    matmul(&mut s.logits, &s.xq, &model.wcls().all());
}
