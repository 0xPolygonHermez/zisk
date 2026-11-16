//! The transformer forward pass, following llama2.c's `runq.c`.

use alloc::vec;
use alloc::vec::Vec;

use crate::fixed;
use crate::model::{Model, QMatrixView};

/// Computes the dot product of two int8 vectors of the same length, as a
/// wrapping `i32` sum. [`forward`] calls it once per group in every matrix
/// multiplication.
pub type DotFn = fn(&[i8], &[i8]) -> i32;

/// The dot product in plain Rust (llama2.c's inner loop).
///
/// It sums in `i64` and saturates to `i32` at the end. With 32-bit sums the
/// compiler uses `addw`, which ZisK proves with the Binary machine instead of
/// the cheaper BinaryAdd, and it narrows an `i64` sum back to `addw` when the
/// result is just truncated. Each product is at most 2^14 in absolute value,
/// so the result only saturates, instead of wrapping like the `dot_i8`
/// precompile, for vectors longer than 131,072 elements.
pub fn dot_soft(x: &[i8], y: &[i8]) -> i32 {
    let mut acc: i64 = 0;
    for (&a, &b) in x.iter().zip(y) {
        acc += a as i64 * b as i64;
    }
    acc.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// How the matrix multiplications combine each group's dot product with the
/// group's two scales.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scales {
    /// In `f32`, as llama2.c: `val += ival · w_scale · x_scale` per group.
    Float,
    /// In fixed point (see [`crate::fixed`]): an exact `i64` sum per row,
    /// converted to `f32` once.
    Fixed,
}

/// Computes one output of a matrix multiplication with fixed-point scales:
/// `Σ_g dot(w_g, x_g) · w_scales[g] · x_scales[g]` over the groups of `gs`
/// values of a row `w` and the vector `x`, exactly in `i64` (see
/// [`crate::fixed`]).
pub type RowFixedFn = fn(w: &[i8], x: &[i8], gs: usize, w_scales: &[i32], x_scales: &[i32]) -> i64;

/// [`RowFixedFn`] in plain Rust, with [`dot_soft`] per group.
pub fn row_fixed_soft(w: &[i8], x: &[i8], gs: usize, w_scales: &[i32], x_scales: &[i32]) -> i64 {
    let mut acc: i64 = 0;
    for ((wg, xg), (&ws, &xs)) in
        w.chunks_exact(gs).zip(x.chunks_exact(gs)).zip(w_scales.iter().zip(x_scales))
    {
        acc += dot_soft(wg, xg) as i64 * (ws as i64 * xs as i64);
    }
    acc
}

/// How the matrix multiplications are computed.
#[derive(Clone, Copy)]
pub struct MatmulMode {
    /// The dot product of one group, used with [`Scales::Float`].
    pub dot: DotFn,
    /// One output, used with [`Scales::Fixed`].
    pub row_fixed: RowFixedFn,
    pub scales: Scales,
}

impl MatmulMode {
    /// Plain Rust, with the given scales.
    pub fn soft(scales: Scales) -> Self {
        MatmulMode { dot: dot_soft, row_fixed: row_fixed_soft, scales }
    }
}

impl Default for MatmulMode {
    /// llama2.c's: [`dot_soft`] and [`Scales::Float`].
    fn default() -> Self {
        Self::soft(Scales::Float)
    }
}

/// An activation vector quantized to int8 in groups, like a matrix row.
///
/// The int8 values are stored in `u64` words, so they are 8-byte aligned, as
/// the `dot_i8` precompile requires.
struct QVec {
    words: Vec<u64>,
    n: usize,
    scales: Vec<f32>,
    /// The scales in fixed point, `scales[g] ≈ fixed_scales[g] · 2^-shift`,
    /// when quantized for [`Scales::Fixed`].
    fixed_scales: Vec<i32>,
    shift: i32,
}

impl QVec {
    fn new(n: usize) -> Self {
        let words = vec![0; n.div_ceil(8)];
        QVec { words, n, scales: vec![0.0; n], fixed_scales: vec![0; n], shift: 0 }
    }

    fn q(&self) -> &[i8] {
        // SAFETY: the words hold at least `n` bytes, and any byte is a valid i8.
        unsafe { core::slice::from_raw_parts(self.words.as_ptr().cast::<i8>(), self.n) }
    }

    fn q_mut(&mut self) -> &mut [i8] {
        // SAFETY: as in `q`, and `&mut self` makes the borrow unique.
        unsafe { core::slice::from_raw_parts_mut(self.words.as_mut_ptr().cast::<i8>(), self.n) }
    }

    /// Quantizes `x` in groups of `gs` (llama2.c's `quantize`), and converts
    /// the scales to fixed point for [`Scales::Fixed`].
    fn quantize(&mut self, x: &[f32], gs: usize, scales: Scales) {
        for (g, group) in x.chunks_exact(gs).enumerate() {
            let wmax = group.iter().fold(0.0f32, |m, &v| m.max(libm::fabsf(v)));
            let scale = wmax / 127.0;
            self.scales[g] = scale;
            let q = &mut self.q_mut()[g * gs..(g + 1) * gs];
            for (qi, &v) in q.iter_mut().zip(group) {
                let qv = if scale == 0.0 { 0.0 } else { libm::roundf(v / scale) };
                *qi = qv as i8;
            }
        }
        if scales == Scales::Fixed {
            let groups = x.len() / gs;
            self.shift =
                fixed::scales_to_fixed(&self.scales[..groups], &mut self.fixed_scales[..groups]);
        }
    }
}

/// `out = W x`, with integer dot products per group (llama2.c's `matmul`).
fn matmul(out: &mut [f32], x: &QVec, w: &QMatrixView, mode: MatmulMode) {
    match mode.scales {
        Scales::Float => matmul_float(out, x, w, mode.dot),
        Scales::Fixed => matmul_fixed(out, x, w, mode.row_fixed),
    }
}

/// [`matmul`] with `f32` scales.
fn matmul_float(out: &mut [f32], x: &QVec, w: &QMatrixView, dot: DotFn) {
    let (n, gs) = (w.cols, w.gs);
    let xq = x.q();
    for (i, o) in out.iter_mut().enumerate().take(w.rows) {
        let row = &w.q[i * n..(i + 1) * n];
        let row_scales = &w.scales[i * n / gs..(i + 1) * n / gs];
        let mut val = 0.0f32;
        for (g, (wg, xg)) in row.chunks_exact(gs).zip(xq.chunks_exact(gs)).enumerate() {
            let ival = dot(wg, xg);
            val += ival as f32 * row_scales[g] * x.scales[g];
        }
        *o = val;
    }
}

/// [`matmul`] with fixed-point scales: no float operation per group.
fn matmul_fixed(out: &mut [f32], x: &QVec, w: &QMatrixView, row_fixed: RowFixedFn) {
    let (n, gs) = (w.cols, w.gs);
    let xq = x.q();
    let x_scales = &x.fixed_scales[..n / gs];
    for (i, o) in out.iter_mut().enumerate().take(w.rows) {
        let row = &w.q[i * n..(i + 1) * n];
        let row_scales = &w.fixed_scales[i * n / gs..(i + 1) * n / gs];
        let acc = row_fixed(row, xq, gs, row_scales, x_scales);
        *o = fixed::fixed_to_f32(acc, w.row_shift[i] + x.shift);
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
    /// How the matrix multiplications are computed.
    mode: MatmulMode,
}

impl RunState {
    /// Computes the matrix multiplications as llama2.c ([`MatmulMode::default`]).
    pub fn new(model: &Model) -> Self {
        Self::with_mode(model, MatmulMode::default())
    }

    pub fn with_mode(model: &Model, mode: MatmulMode) -> Self {
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
            mode,
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
        s.xq.quantize(&s.xb, gs_dim, s.mode.scales);
        matmul(&mut s.q, &s.xq, &model.wq.rows(l * dim, dim), s.mode);
        matmul(&mut s.k, &s.xq, &model.wk.rows(l * kv_dim, kv_dim), s.mode);
        matmul(&mut s.v, &s.xq, &model.wv.rows(l * kv_dim, kv_dim), s.mode);

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

        s.xq.quantize(&s.xb, gs_dim, s.mode.scales);
        matmul(&mut s.xb2, &s.xq, &model.wo.rows(l * dim, dim), s.mode);
        for (x, &d) in s.x.iter_mut().zip(&s.xb2) {
            *x += d;
        }

        // Feed-forward: w2(silu(w1 x) * w3 x).
        rmsnorm(&mut s.xb, &s.x, &model.rms_ffn[l * dim..(l + 1) * dim]);
        s.xq.quantize(&s.xb, gs_dim, s.mode.scales);
        matmul(&mut s.hb, &s.xq, &model.w1.rows(l * hidden, hidden), s.mode);
        matmul(&mut s.hb2, &s.xq, &model.w3.rows(l * hidden, hidden), s.mode);
        for (h, &h2) in s.hb.iter_mut().zip(&s.hb2) {
            let mut val = *h;
            val *= 1.0 / (1.0 + libm::expf(-val));
            *h = val * h2;
        }
        s.hq.quantize(&s.hb, gs_hidden, s.mode.scales);
        matmul(&mut s.xb, &s.hq, &model.w2.rows(l * dim, dim), s.mode);
        for (x, &d) in s.x.iter_mut().zip(&s.xb) {
            *x += d;
        }
    }

    let x = s.x.clone();
    rmsnorm(&mut s.x, &x, &model.rms_final);
    s.xq.quantize(&s.x, gs_dim, s.mode.scales);
    matmul(&mut s.logits, &s.xq, &model.wcls().all(), s.mode);
}
