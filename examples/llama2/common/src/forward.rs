//! The transformer forward pass, following llama2.c's `runq.c`.

use alloc::vec;
use alloc::vec::Vec;

use crate::fixed;
use crate::formats::{Format, quantize_group};
use crate::model::{Model, QMatrixView};

/// Computes the exact dot products of one matrix row `w` (format `wf`) and a
/// vector `x` (format `xf`), both of `n` values, in groups of `gs`: writes the
/// sum of each group to `out`, in units of the two formats (see
/// [`crate::formats`]). This is what the `dot_f8` precompile computes in one
/// call; [`forward`] calls it once per row of every matrix multiplication.
pub type DotFn =
    fn(w: &[u8], wf: Format, x: &[u8], xf: Format, n: usize, gs: usize, out: &mut [i64]);

/// [`DotFn`] in plain Rust.
///
/// For int8 this is llama2.c's inner loop. It sums in `i64`: with 32-bit
/// sums the compiler uses `addw`, which ZisK proves with the Binary machine
/// instead of the cheaper BinaryAdd. The other formats decode each value with
/// a table lookup.
pub fn dot_soft(w: &[u8], wf: Format, x: &[u8], xf: Format, n: usize, gs: usize, out: &mut [i64]) {
    if wf == Format::Int8 && xf == Format::Int8 {
        for ((wg, xg), o) in w[..n].chunks_exact(gs).zip(x[..n].chunks_exact(gs)).zip(out) {
            let mut acc: i64 = 0;
            for (&a, &b) in wg.iter().zip(xg) {
                acc += a as i8 as i64 * b as i8 as i64;
            }
            *o = acc;
        }
    } else {
        zisk_definitions::dot_f8_reference(wf.id(), w, xf.id(), x, n, gs, Some(out));
    }
}

/// Computes the dot product of a matrix row `w` and a vector `x` of BF16
/// values (as `u16` bits), exactly, rounded once to `f32`. This is what the
/// `dot_bf16` precompile computes; [`forward`] calls it once per row of every
/// matrix multiplication of a BF16 model.
pub type DotBf16Fn = fn(w: &[u16], x: &[u16]) -> f32;

/// [`DotBf16Fn`] in plain Rust: the reference semantics, with a 576-bit exact
/// accumulator.
pub fn dot_bf16_soft(w: &[u16], x: &[u16]) -> f32 {
    f32::from_bits(zisk_definitions::dot_bf16_reference(w, x, w.len(), 0, None))
}

/// Computes the dot product of a matrix row of FP4 weights (`x`, packed E2M1 values, with
/// `scales` one byte per block) and a vector `y` of BF16 values, exactly, rounded once to `f32`.
/// `flags` selects MXFP4 or NVFP4 (`zisk_definitions::DOT_BF16_FLAGS_*`); NVFP4's tensor scale
/// is not included. This is what the `dot_bf16` precompile computes with FP4 weights.
pub type DotBf16Fp4Fn = fn(flags: u64, x: &[u8], scales: &[u8], y: &[u16]) -> f32;

/// [`DotBf16Fp4Fn`] in plain Rust: the reference semantics.
pub fn dot_bf16_fp4_soft(flags: u64, x: &[u8], scales: &[u8], y: &[u16]) -> f32 {
    let n = y.len();
    f32::from_bits(zisk_definitions::dot_bf16_fp4_reference(flags, x, scales, y, n, 0, None))
}

/// The bytes of BF16 values as `u16`. They are 2-byte aligned: rows of the
/// blob start at multiples of 8 bytes, and activations are stored in `u64`.
fn as_bf16(bytes: &[u8]) -> &[u16] {
    // SAFETY: any two bytes are a valid u16; align_to checks the alignment.
    let (head, values, tail) = unsafe { bytes.align_to::<u16>() };
    assert!(head.is_empty() && tail.is_empty(), "BF16 values must be 2-byte aligned");
    values
}

/// How the matrix multiplications combine each group's dot product with the
/// group's two scales.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scales {
    /// In `f32`, as llama2.c: `val += ival · w_scale · x_scale` per group.
    Float,
    /// In fixed point (see [`crate::fixed`]): an exact integer sum per row,
    /// converted to `f32` once.
    Fixed,
}

/// How the matrix multiplications are computed.
#[derive(Clone, Copy)]
pub struct MatmulMode {
    /// The dot products of one row.
    pub dot: DotFn,
    pub scales: Scales,
    /// The dot product of one row of a BF16 model, whatever `scales` is.
    pub dot_bf16: DotBf16Fn,
    /// The dot product of one row of MXFP4 or NVFP4 weights and BF16 activations.
    pub dot_bf16_fp4: DotBf16Fp4Fn,
}

impl MatmulMode {
    /// Plain Rust, with the given scales.
    pub fn soft(scales: Scales) -> Self {
        MatmulMode {
            dot: dot_soft,
            scales,
            dot_bf16: dot_bf16_soft,
            dot_bf16_fp4: dot_bf16_fp4_soft,
        }
    }
}

impl Default for MatmulMode {
    /// llama2.c's: [`dot_soft`] and [`Scales::Float`].
    fn default() -> Self {
        Self::soft(Scales::Float)
    }
}

/// An activation vector quantized in groups, like a matrix row, in the
/// model's activation format.
///
/// The values are stored in `u64` words, so they are 8-byte aligned, as the
/// `dot_f8` precompile requires.
struct QVec {
    words: Vec<u64>,
    n: usize,
    format: Format,
    /// The scale of one unit of each group.
    scales: Vec<f32>,
    /// The scales in fixed point, `scales[g] ≈ fixed_scales[g] · 2^-shift`,
    /// when quantized for [`Scales::Fixed`].
    fixed_scales: Vec<i32>,
    shift: i32,
}

impl QVec {
    fn new(n: usize, format: Format) -> Self {
        let words = vec![0; Self::len(n, format).div_ceil(8)];
        QVec { words, n, format, scales: vec![0.0; n], fixed_scales: vec![0; n], shift: 0 }
    }

    /// Bytes of the buffer: at least `n`, as `dot_soft` indexes int8 values,
    /// and the 2 bytes per value of BF16.
    fn len(n: usize, format: Format) -> usize {
        n.max(format.bytes(n))
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the words hold at least `len` bytes.
        unsafe {
            core::slice::from_raw_parts(
                self.words.as_ptr().cast::<u8>(),
                Self::len(self.n, self.format),
            )
        }
    }

    /// Quantizes `x` in groups of `gs` (llama2.c's `quantize` for int8), and
    /// converts the scales to fixed point for [`Scales::Fixed`].
    fn quantize(&mut self, x: &[f32], gs: usize, scales: Scales) {
        let f = self.format;
        // SAFETY: as in `bytes`, and `&mut self` makes the borrow unique.
        let q = unsafe {
            core::slice::from_raw_parts_mut(
                self.words.as_mut_ptr().cast::<u8>(),
                Self::len(self.n, self.format),
            )
        };
        for (g, group) in x.chunks_exact(gs).enumerate() {
            self.scales[g] =
                quantize_group(f, group, &mut q[f.bytes(g * gs)..f.bytes((g + 1) * gs)]);
        }
        if scales == Scales::Fixed {
            let groups = x.len() / gs;
            self.shift =
                fixed::scales_to_fixed(&self.scales[..groups], &mut self.fixed_scales[..groups]);
        }
    }
}

/// `out = W x`, with exact dot products per group (llama2.c's `matmul`).
/// `sums` holds the group sums of one row.
fn matmul(out: &mut [f32], x: &QVec, w: &QMatrixView, mode: MatmulMode, sums: &mut [i64]) {
    if w.format == Format::Bf16 {
        return matmul_bf16(out, x, w, mode.dot_bf16);
    }
    if w.format.is_fp4_weights() {
        return matmul_fp4(out, x, w, mode.dot_bf16_fp4);
    }
    let (n, gs) = (w.cols, w.gs);
    let (wf, xq) = (w.format, x.bytes());
    let groups = n / gs;
    let sums = &mut sums[..groups];
    let x_fixed = &x.fixed_scales[..groups];
    // int8 sums fit an i64 with the scales applied (see fixed::MAX_LEN); the
    // floats need an i128.
    let wide = wf != Format::Int8 || x.format != Format::Int8;
    // Bytes per row, computed once: Format::bytes matches on the format, which per row costs
    // steps in ZisK
    let row_bytes = wf.bytes(n);
    for (i, o) in out.iter_mut().enumerate().take(w.rows) {
        let row = &w.q[i * row_bytes..(i + 1) * row_bytes];
        (mode.dot)(row, wf, xq, x.format, n, gs, sums);
        let (g0, g1) = (i * groups, (i + 1) * groups);
        *o = match mode.scales {
            Scales::Float => {
                let mut val = 0.0f32;
                for ((&ival, &ws), &xs) in sums.iter().zip(&w.scales[g0..g1]).zip(&x.scales) {
                    val += ival as f32 * ws * xs;
                }
                val
            }
            Scales::Fixed if !wide => {
                let mut acc: i64 = 0;
                for ((&ival, &ws), &xs) in sums.iter().zip(&w.fixed_scales[g0..g1]).zip(x_fixed) {
                    acc += ival * (ws as i64 * xs as i64);
                }
                fixed::fixed_to_f32(acc, w.row_shift[i] + x.shift)
            }
            Scales::Fixed => {
                let mut acc: i128 = 0;
                for ((&ival, &ws), &xs) in sums.iter().zip(&w.fixed_scales[g0..g1]).zip(x_fixed) {
                    acc += ival as i128 * (ws as i64 * xs as i64) as i128;
                }
                fixed::wide_fixed_to_f32(acc, w.row_shift[i] + x.shift)
            }
        };
    }
}

/// [`matmul`] for BF16 weights and activations: one exact dot product per
/// row, rounded once to `f32`, with no scales.
fn matmul_bf16(out: &mut [f32], x: &QVec, w: &QMatrixView, dot: DotBf16Fn) {
    let n = w.cols;
    let xq = &as_bf16(x.bytes())[..n];
    let wq = as_bf16(w.q);
    for (i, o) in out.iter_mut().enumerate().take(w.rows) {
        *o = dot(&wq[i * n..(i + 1) * n], xq);
    }
}

/// [`matmul`] for MXFP4 or NVFP4 weights and BF16 activations: one exact dot product per row,
/// rounded once to `f32`, times NVFP4's tensor scale (1 for MXFP4).
fn matmul_fp4(out: &mut [f32], x: &QVec, w: &QMatrixView, dot: DotBf16Fp4Fn) {
    let n = w.cols;
    let flags = w.format.dot_bf16_flags();
    let xq = &as_bf16(x.bytes())[..n];
    let blocks = n / w.format.fp4_block();
    for (i, o) in out.iter_mut().enumerate().take(w.rows) {
        let row = &w.q[i * n / 2..(i + 1) * n / 2];
        let scales = &w.block_scales[i * w.scale_stride..i * w.scale_stride + blocks];
        let v = dot(flags, row, scales, xq);
        *o = if w.format == Format::Nvfp4 { v * w.scales[i] } else { v };
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
    /// The group sums of one matrix row.
    sums: Vec<i64>,
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
            xq: QVec::new(dim, model.activations),
            hq: QVec::new(hidden, model.activations),
            sums: vec![0; dim.max(hidden)],
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
    let row = &e.q[e.format.bytes(t * dim)..e.format.bytes((t + 1) * dim)];
    if e.format == Format::Bf16 {
        for (x, &b) in s.x.iter_mut().zip(as_bf16(row)) {
            *x = zisk_definitions::bf16_to_f32(b);
        }
    } else if e.format.is_fp4_weights() {
        let scales = &e.block_scales[t * e.scale_stride..(t + 1) * e.scale_stride];
        let tensor = if e.format == Format::Nvfp4 { e.scales[t] } else { 1.0 };
        for (i, x) in s.x.iter_mut().enumerate() {
            *x = crate::formats::fp4_weight(e.format, row, scales, i) * tensor;
        }
    } else {
        for i in 0..dim {
            s.x[i] = e.format.units(row, i) as f32 * e.scales[(t * dim + i) / e.gs];
        }
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
        matmul(&mut s.q, &s.xq, &model.wq.rows(l * dim, dim), s.mode, &mut s.sums);
        matmul(&mut s.k, &s.xq, &model.wk.rows(l * kv_dim, kv_dim), s.mode, &mut s.sums);
        matmul(&mut s.v, &s.xq, &model.wv.rows(l * kv_dim, kv_dim), s.mode, &mut s.sums);

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
        matmul(&mut s.xb2, &s.xq, &model.wo.rows(l * dim, dim), s.mode, &mut s.sums);
        for (x, &d) in s.x.iter_mut().zip(&s.xb2) {
            *x += d;
        }

        // Feed-forward: w2(silu(w1 x) * w3 x).
        rmsnorm(&mut s.xb, &s.x, &model.rms_ffn[l * dim..(l + 1) * dim]);
        s.xq.quantize(&s.xb, gs_dim, s.mode.scales);
        matmul(&mut s.hb, &s.xq, &model.w1.rows(l * hidden, hidden), s.mode, &mut s.sums);
        matmul(&mut s.hb2, &s.xq, &model.w3.rows(l * hidden, hidden), s.mode, &mut s.sums);
        for (h, &h2) in s.hb.iter_mut().zip(&s.hb2) {
            let mut val = *h;
            val *= 1.0 / (1.0 + libm::expf(-val));
            *h = val * h2;
        }
        s.hq.quantize(&s.hb, gs_hidden, s.mode.scales);
        matmul(&mut s.xb, &s.hq, &model.w2.rows(l * dim, dim), s.mode, &mut s.sums);
        for (x, &d) in s.x.iter_mut().zip(&s.xb) {
            *x += d;
        }
    }

    let x = s.x.clone();
    rmsnorm(&mut s.x, &x, &model.rms_final);
    s.xq.quantize(&s.x, gs_dim, s.mode.scales);
    matmul(&mut s.logits, &s.xq, &model.wcls().all(), s.mode, &mut s.sums);
}
