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
//! f32 rms_att[layers*dim], rms_ffn[layers*dim], rms_final[dim]
//! qtensor token_embedding, wq, wk, wv, wo, w1, w2, w3, [wcls]
//! ```
//!
//! A qtensor is `i8 q[rows*cols]` followed by `f32 scale[rows*cols/gs]`. The
//! per-layer matrices are stored as one qtensor each, layers stacked by rows.

use alloc::vec::Vec;

use crate::{read_f32, read_u32};

pub const MAGIC: u32 = u32::from_le_bytes(*b"L2Q8");
pub const VERSION: u32 = 1;

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

/// Largest power-of-two group size, at most 64, that divides `n`.
pub fn group_size(n: usize) -> usize {
    let mut gs = 64;
    while n % gs != 0 {
        gs /= 2;
    }
    gs
}

/// Quantizes `w` in groups of `gs`: each group gets `scale = max|w| / 127`
/// and `q = round(w / scale)`, as llama2.c's `quantize`.
pub fn quantize_into(w: &[f32], gs: usize, q: &mut Vec<u8>, scales: &mut Vec<f32>) {
    for group in w.chunks_exact(gs) {
        let wmax = group.iter().fold(0.0f32, |m, &v| m.max(libm::fabsf(v)));
        let scale = wmax / 127.0;
        scales.push(scale);
        for &v in group {
            let qv = if scale == 0.0 { 0.0 } else { libm::roundf(v / scale) };
            q.push(qv as i8 as u8);
        }
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

fn push_qtensor(blob: &mut Vec<u8>, w: &[f32], gs: usize) {
    let mut q = Vec::with_capacity(w.len());
    let mut scales = Vec::with_capacity(w.len() / gs);
    quantize_into(w, gs, &mut q, &mut scales);
    blob.extend_from_slice(&q);
    push_floats(blob, &scales);
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

/// Quantizes a llama2.c float32 checkpoint into a model blob.
pub fn quantize_checkpoint(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    let (c, shared) = read_config(bytes)?;
    let (dim, hidden, layers, kv_dim) = (c.dim, c.hidden_dim, c.n_layers, c.kv_dim());
    let (gs_dim, gs_hidden) = (group_size(dim), group_size(hidden));

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
    push_floats(&mut blob, &rms_att);
    push_floats(&mut blob, &rms_ffn);
    push_floats(&mut blob, &rms_final);
    push_qtensor(&mut blob, &embedding, gs_dim);
    push_qtensor(&mut blob, &wq, gs_dim);
    push_qtensor(&mut blob, &wk, gs_dim);
    push_qtensor(&mut blob, &wv, gs_dim);
    push_qtensor(&mut blob, &wo, gs_dim);
    push_qtensor(&mut blob, &w1, gs_dim);
    push_qtensor(&mut blob, &w2, gs_hidden);
    push_qtensor(&mut blob, &w3, gs_dim);
    if let Some(wcls) = &wcls {
        push_qtensor(&mut blob, wcls, gs_dim);
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
    fn quantization_keeps_the_largest_value_exact() {
        let w = [0.5f32, -1.0, 0.25, 0.0];
        let (mut q, mut scales) = (Vec::new(), Vec::new());
        quantize_into(&w, 4, &mut q, &mut scales);
        assert_eq!(scales, [1.0 / 127.0]);
        assert_eq!(q.iter().map(|&b| b as i8).collect::<Vec<_>>(), [64, -127, 32, 0]);
    }
}
