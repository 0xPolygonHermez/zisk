//! Reads a llama2.c float32 checkpoint and quantizes it into a model blob.
//!
//! Checkpoint layout (llama2.c "legacy" export, as used by the TinyStories
//! models): seven `i32` (dim, hidden_dim, n_layers, n_heads, n_kv_heads,
//! vocab_size, seq_len; a negative vocab_size means the classifier is not
//! shared with the token embedding), then `f32` tensors:
//!
//! ```text
//! token_embedding [vocab][dim]
//! rms_att [layers][dim]
//! wq [layers][dim][dim]      wk [layers][kv_dim][dim]   wv [layers][kv_dim][dim]
//! wo [layers][dim][dim]
//! rms_ffn [layers][dim]
//! w1 [layers][hidden][dim]   w2 [layers][dim][hidden]   w3 [layers][hidden][dim]
//! rms_final [dim]
//! freq_cis_real, freq_cis_imag [seq_len][head_size / 2]   (unused)
//! wcls [vocab][dim]          (only when not shared)
//! ```
//!
//! Model blob layout (little-endian), read by [`crate::model::Model::parse`]:
//!
//! ```text
//! u32 magic "L2Q8", u32 version
//! u32 dim, hidden_dim, n_layers, n_heads, n_kv_heads, vocab_size, seq_len
//! u32 shared_classifier (0 or 1)
//! u32 gs_dim, gs_hidden       group sizes for inputs of width dim / hidden_dim
//! u32 weights, activations    formats of the weights and of the quantized
//!                             activations ([`Format::id`]: 0 int8, 1 E4M3,
//!                             2 E4M3FNUZ, 3 E2M1, 16 BF16; weights only:
//!                             17 MXFP4, 18 NVFP4)
//! f32 rms_att[layers*dim], rms_ffn[layers*dim], rms_final[dim]
//! qtensor token_embedding, wq, wk, wv, wo, w1, w2, w3, [wcls]
//! ```
//!
//! A qtensor is the `rows*cols` values in the weight format (one byte each,
//! or two per byte for E2M1), then `f32 scale[rows*cols/gs]`, each part padded
//! with zeros to a multiple of 8 bytes so that matrix rows stay 8-byte
//! aligned, as the `dot_f8` precompile needs. Each scale is the value of one
//! unit of its group (see [`crate::formats`]). The per-layer matrices are
//! stored as one qtensor each, layers stacked by rows.
//!
//! With MXFP4 or NVFP4 weights, a qtensor is the packed E2M1 values (padded to
//! 8 bytes), then the block scales of each row, one byte per block of 32
//! (MXFP4) or 16 (NVFP4), each row's padded with zeros to a multiple of 8
//! bytes so that it can be passed to the precompile, then one `f32` per row:
//! 1 for MXFP4, the tensor scale of the row's layer for NVFP4.

use alloc::vec::Vec;

use crate::formats::{Format, quantize_group};
use crate::{read_f32, read_u32};

pub const MAGIC: u32 = u32::from_le_bytes(*b"L2Q8");
pub const VERSION: u32 = 2;

/// How a model is quantized: the formats of the weights and of the
/// activations they multiply, and the largest group size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quantization {
    pub weights: Format,
    pub activations: Format,
    /// Groups of at most this many values ([`group_size_at_most`]), or
    /// llama2.c's choice ([`group_size`]).
    pub max_group: Option<usize>,
}

impl Default for Quantization {
    /// llama2.c's: int8 weights and activations, its group sizes.
    fn default() -> Self {
        Quantization { weights: Format::Int8, activations: Format::Int8, max_group: None }
    }
}

impl Quantization {
    /// Group sizes must hold whole 64-bit words of both formats for the
    /// `dot_f8` precompile: 8 values, or 16 with E2M1.
    pub fn group_multiple(&self) -> usize {
        64 / self.weights.bits().min(self.activations.bits())
    }

    /// The group size for rows of `n` values: the whole row for BF16, which
    /// has no scales.
    pub fn group_size(&self, n: usize) -> usize {
        if self.weights.uses_bf16() {
            return n;
        }
        match self.max_group {
            Some(m) => group_size_at_most(n, m, self.group_multiple()),
            None => group_size(n),
        }
    }
}

/// Model hyper-parameters, as in llama2.c's `Config`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub dim: usize,
    pub hidden_dim: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub vocab_size: usize,
    pub seq_len: usize,
}

impl Config {
    pub fn head_size(&self) -> usize {
        self.dim / self.n_heads
    }

    pub fn kv_dim(&self) -> usize {
        self.dim * self.n_kv_heads / self.n_heads
    }
}

/// Largest power-of-two group size, at most 64, that divides `n` (llama2.c's
/// choice).
pub fn group_size(n: usize) -> usize {
    let mut gs = 64;
    while n % gs != 0 {
        gs /= 2;
    }
    gs
}

/// Largest group size, at most `max`, that divides `n` and is a multiple of
/// `multiple` (8, or 16 with E2M1), as the `dot_f8` precompile needs. Falls
/// back to [`group_size`] when there is none.
pub fn group_size_at_most(n: usize, max: usize, multiple: usize) -> usize {
    (multiple..=max.min(n))
        .rev()
        .find(|&d| d % multiple == 0 && n % d == 0)
        .unwrap_or(group_size(n))
}

/// Quantizes `w` in groups of `gs` values of format `f` ([`quantize_group`]),
/// appending their bytes to `q` and their scales to `scales`. For int8 this is
/// llama2.c's `quantize`.
pub fn quantize_into(f: Format, w: &[f32], gs: usize, q: &mut Vec<u8>, scales: &mut Vec<f32>) {
    for group in w.chunks_exact(gs) {
        let start = q.len();
        q.resize(start + f.bytes(gs), 0);
        scales.push(quantize_group(f, group, &mut q[start..]));
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn floats(&mut self, n: usize) -> Result<Vec<f32>, &'static str> {
        let end = self.pos + 4 * n;
        if end > self.bytes.len() {
            return Err("checkpoint is truncated");
        }
        let v = (0..n).map(|i| read_f32(self.bytes, self.pos + 4 * i)).collect();
        self.pos = end;
        Ok(v)
    }

    fn skip(&mut self, n_floats: usize) {
        self.pos += 4 * n_floats;
    }
}

fn push_u32(blob: &mut Vec<u8>, v: usize) {
    blob.extend_from_slice(&(v as u32).to_le_bytes());
}

fn push_floats(blob: &mut Vec<u8>, v: &[f32]) {
    for x in v {
        blob.extend_from_slice(&x.to_le_bytes());
    }
}

fn pad8(blob: &mut Vec<u8>) {
    blob.resize(blob.len().next_multiple_of(8), 0);
}

/// Bytes of the block scales of one row of `cols` FP4 weights, padded to whole words.
pub fn fp4_scale_stride(f: Format, cols: usize) -> usize {
    (cols / f.fp4_block()).next_multiple_of(8)
}

/// Appends a matrix of FP4 weights (see the module documentation), with one NVFP4 tensor scale
/// per `tensor_rows` rows (one layer's matrix).
fn push_fp4_qtensor(blob: &mut Vec<u8>, w: &[f32], cols: usize, f: Format, tensor_rows: usize) {
    let rows = w.len() / cols;
    let stride = fp4_scale_stride(f, cols);
    let mut q = alloc::vec![0u8; w.len() / 2];
    let mut scales = alloc::vec![0u8; rows * stride];
    let mut row_scales = Vec::with_capacity(rows);
    for (t, tensor) in w.chunks(tensor_rows * cols).enumerate() {
        let ts = if f == Format::Nvfp4 { crate::formats::nvfp4_tensor_scale(tensor) } else { 1.0 };
        for (k, row) in tensor.chunks_exact(cols).enumerate() {
            let r = t * tensor_rows + k;
            let (qr, sr) = (&mut q[r * cols / 2..(r + 1) * cols / 2], &mut scales[r * stride..]);
            if f == Format::Mxfp4 {
                crate::formats::quantize_mxfp4_row(row, qr, sr);
            } else {
                crate::formats::quantize_nvfp4_row(row, ts, qr, sr);
            }
            row_scales.push(ts);
        }
    }
    blob.extend_from_slice(&q);
    pad8(blob);
    blob.extend_from_slice(&scales);
    push_floats(blob, &row_scales);
    pad8(blob);
}

/// Appends a matrix of `rows · cols` weights; `tensor_rows` rows make one layer's tensor.
fn push_qtensor(blob: &mut Vec<u8>, w: &[f32], gs: usize, f: Format, tensor_rows: usize) {
    if f.is_fp4_weights() {
        return push_fp4_qtensor(blob, w, gs, f, tensor_rows);
    }
    let mut q = Vec::with_capacity(f.bytes(w.len()));
    let mut scales = Vec::with_capacity(w.len() / gs);
    quantize_into(f, w, gs, &mut q, &mut scales);
    blob.extend_from_slice(&q);
    pad8(blob);
    push_floats(blob, &scales);
    pad8(blob);
}

/// Reads the header of a llama2.c checkpoint.
pub fn read_config(bytes: &[u8]) -> Result<(Config, bool), &'static str> {
    if bytes.len() < 28 {
        return Err("checkpoint is too short");
    }
    let h: Vec<i32> = (0..7).map(|i| read_u32(bytes, 4 * i) as i32).collect();
    if h.iter().enumerate().any(|(i, &v)| v == 0 || (i != 5 && v < 0)) {
        return Err("checkpoint header is not a llama2.c config");
    }
    let config = Config {
        dim: h[0] as usize,
        hidden_dim: h[1] as usize,
        n_layers: h[2] as usize,
        n_heads: h[3] as usize,
        n_kv_heads: h[4] as usize,
        vocab_size: h[5].unsigned_abs() as usize,
        seq_len: h[6] as usize,
    };
    Ok((config, h[5] > 0))
}

/// Quantizes a llama2.c float32 checkpoint into a model blob, as llama2.c:
/// int8 and its group sizes ([`group_size`]).
pub fn quantize_checkpoint(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    quantize_checkpoint_with(bytes, Quantization::default())
}

/// [`quantize_checkpoint`], with the given formats and group sizes.
pub fn quantize_checkpoint_with(bytes: &[u8], qz: Quantization) -> Result<Vec<u8>, &'static str> {
    if qz.weights.uses_bf16() != (qz.activations == Format::Bf16) || qz.activations.is_fp4_weights()
    {
        return Err("BF16, MXFP4 and NVFP4 weights need BF16 activations, and BF16 activations \
                    need one of them");
    }
    let (c, shared) = read_config(bytes)?;
    if qz.weights.is_fp4_weights()
        && (c.dim % qz.weights.fp4_block() != 0 || c.hidden_dim % qz.weights.fp4_block() != 0)
    {
        return Err("MXFP4 and NVFP4 need dim and hidden_dim to be multiples of their block (32 \
                    or 16)");
    }
    let (dim, hidden, layers, kv_dim) = (c.dim, c.hidden_dim, c.n_layers, c.kv_dim());
    let (gs_dim, gs_hidden) = (qz.group_size(dim), qz.group_size(hidden));
    let format = qz.weights;

    let mut r = Reader { bytes, pos: 28 };
    let embedding = r.floats(c.vocab_size * dim)?;
    let rms_att = r.floats(layers * dim)?;
    let wq = r.floats(layers * dim * dim)?;
    let wk = r.floats(layers * kv_dim * dim)?;
    let wv = r.floats(layers * kv_dim * dim)?;
    let wo = r.floats(layers * dim * dim)?;
    let rms_ffn = r.floats(layers * dim)?;
    let w1 = r.floats(layers * hidden * dim)?;
    let w2 = r.floats(layers * dim * hidden)?;
    let w3 = r.floats(layers * hidden * dim)?;
    let rms_final = r.floats(dim)?;
    r.skip(c.seq_len * c.head_size()); // freq_cis_real and freq_cis_imag
    let wcls = if shared { None } else { Some(r.floats(c.vocab_size * dim)?) };

    let mut blob = Vec::new();
    for v in [MAGIC as usize, VERSION as usize] {
        push_u32(&mut blob, v);
    }
    for v in [dim, hidden, layers, c.n_heads, c.n_kv_heads, c.vocab_size, c.seq_len] {
        push_u32(&mut blob, v);
    }
    push_u32(&mut blob, shared as usize);
    push_u32(&mut blob, gs_dim);
    push_u32(&mut blob, gs_hidden);
    push_u32(&mut blob, qz.weights.id() as usize);
    push_u32(&mut blob, qz.activations.id() as usize);
    push_floats(&mut blob, &rms_att);
    push_floats(&mut blob, &rms_ffn);
    push_floats(&mut blob, &rms_final);
    push_qtensor(&mut blob, &embedding, gs_dim, format, c.vocab_size);
    push_qtensor(&mut blob, &wq, gs_dim, format, dim);
    push_qtensor(&mut blob, &wk, gs_dim, format, kv_dim);
    push_qtensor(&mut blob, &wv, gs_dim, format, kv_dim);
    push_qtensor(&mut blob, &wo, gs_dim, format, dim);
    push_qtensor(&mut blob, &w1, gs_dim, format, hidden);
    push_qtensor(&mut blob, &w2, gs_hidden, format, dim);
    push_qtensor(&mut blob, &w3, gs_dim, format, hidden);
    if let Some(wcls) = &wcls {
        push_qtensor(&mut blob, wcls, gs_dim, format, c.vocab_size);
    }
    Ok(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_size_is_the_largest_power_of_two_up_to_64() {
        assert_eq!(group_size(64), 64);
        assert_eq!(group_size(288), 32);
        assert_eq!(group_size(768), 64);
        assert_eq!(group_size(172), 4);
    }

    #[test]
    fn group_size_at_most_is_a_multiple_of_8() {
        assert_eq!(group_size_at_most(288, 64, 8), 48);
        assert_eq!(group_size_at_most(288, 144, 8), 144);
        assert_eq!(group_size_at_most(288, 100, 8), 96);
        assert_eq!(group_size_at_most(768, 144, 8), 128);
        assert_eq!(group_size_at_most(64, 144, 8), 64);
        assert_eq!(group_size_at_most(172, 144, 8), 4);
        assert_eq!(group_size_at_most(288, 64, 16), 48);
        assert_eq!(group_size_at_most(288, 128, 16), 96);
    }

    #[test]
    fn quantization_keeps_the_largest_value_exact() {
        let w = [0.5f32, -1.0, 0.25, 0.0];
        let (mut q, mut scales) = (Vec::new(), Vec::new());
        quantize_into(Format::Int8, &w, 4, &mut q, &mut scales);
        assert_eq!(scales, [1.0 / 127.0]);
        assert_eq!(q.iter().map(|&b| b as i8).collect::<Vec<_>>(), [64, -127, 32, 0]);
    }
}
