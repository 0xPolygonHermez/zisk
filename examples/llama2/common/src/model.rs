//! Parses a model blob (see [`crate::checkpoint`]) without copying the int8
//! weights; only the small `f32` arrays are copied.

use alloc::vec::Vec;

use crate::checkpoint::{Config, MAGIC, VERSION};
use crate::{read_f32, read_u32};

/// An int8 matrix of `rows` x `cols`, quantized in groups of `gs` along each row.
pub struct QMatrix<'a> {
    pub rows: usize,
    pub cols: usize,
    pub gs: usize,
    pub q: &'a [i8],
    pub scales: Vec<f32>,
}

impl QMatrix<'_> {
    /// Rows `[first, first + n)`, as a view sharing the weights.
    pub fn rows(&self, first: usize, n: usize) -> QMatrixView<'_> {
        QMatrixView {
            rows: n,
            cols: self.cols,
            gs: self.gs,
            q: &self.q[first * self.cols..(first + n) * self.cols],
            scales: &self.scales[first * self.cols / self.gs..(first + n) * self.cols / self.gs],
        }
    }

    pub fn all(&self) -> QMatrixView<'_> {
        self.rows(0, self.rows)
    }
}

/// A borrowed range of rows of a [`QMatrix`].
pub struct QMatrixView<'a> {
    pub rows: usize,
    pub cols: usize,
    pub gs: usize,
    pub q: &'a [i8],
    pub scales: &'a [f32],
}

/// The parsed model. Per-layer matrices are stacked: layer `l` of `wq` is
/// `wq.rows(l * dim, dim)`.
pub struct Model<'a> {
    pub config: Config,
    pub rms_att: Vec<f32>,
    pub rms_ffn: Vec<f32>,
    pub rms_final: Vec<f32>,
    pub embedding: QMatrix<'a>,
    pub wq: QMatrix<'a>,
    pub wk: QMatrix<'a>,
    pub wv: QMatrix<'a>,
    pub wo: QMatrix<'a>,
    pub w1: QMatrix<'a>,
    pub w2: QMatrix<'a>,
    pub w3: QMatrix<'a>,
    wcls: Option<QMatrix<'a>>,
}

struct Cursor<'a> {
    blob: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn u32(&mut self) -> usize {
        let v = read_u32(self.blob, self.pos);
        self.pos += 4;
        v as usize
    }

    fn floats(&mut self, n: usize) -> Vec<f32> {
        let v = (0..n).map(|i| read_f32(self.blob, self.pos + 4 * i)).collect();
        self.pos += 4 * n;
        v
    }

    fn qmatrix(&mut self, rows: usize, cols: usize, gs: usize) -> QMatrix<'a> {
        let n = rows * cols;
        let bytes = &self.blob[self.pos..self.pos + n];
        // SAFETY: i8 and u8 have the same size and alignment.
        let q = unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast::<i8>(), n) };
        self.pos += n;
        let scales = self.floats(n / gs);
        QMatrix { rows, cols, gs, q, scales }
    }
}

impl<'a> Model<'a> {
    /// Parses a model blob. Panics if the blob is malformed.
    pub fn parse(blob: &'a [u8]) -> Self {
        let mut c = Cursor { blob, pos: 0 };
        assert_eq!(c.u32() as u32, MAGIC, "not a model blob");
        assert_eq!(c.u32() as u32, VERSION, "unsupported model blob version");
        let config = Config {
            dim: c.u32(),
            hidden_dim: c.u32(),
            n_layers: c.u32(),
            n_heads: c.u32(),
            n_kv_heads: c.u32(),
            vocab_size: c.u32(),
            seq_len: c.u32(),
        };
        let shared = c.u32() != 0;
        let (gs_dim, gs_hidden) = (c.u32(), c.u32());
        let (dim, hidden, layers, kv_dim) =
            (config.dim, config.hidden_dim, config.n_layers, config.kv_dim());

        let rms_att = c.floats(layers * dim);
        let rms_ffn = c.floats(layers * dim);
        let rms_final = c.floats(dim);
        let embedding = c.qmatrix(config.vocab_size, dim, gs_dim);
        let wq = c.qmatrix(layers * dim, dim, gs_dim);
        let wk = c.qmatrix(layers * kv_dim, dim, gs_dim);
        let wv = c.qmatrix(layers * kv_dim, dim, gs_dim);
        let wo = c.qmatrix(layers * dim, dim, gs_dim);
        let w1 = c.qmatrix(layers * hidden, dim, gs_dim);
        let w2 = c.qmatrix(layers * dim, hidden, gs_hidden);
        let w3 = c.qmatrix(layers * hidden, dim, gs_dim);
        let wcls = (!shared).then(|| c.qmatrix(config.vocab_size, dim, gs_dim));
        assert_eq!(c.pos, blob.len(), "trailing bytes in the model blob");

        Model { config, rms_att, rms_ffn, rms_final, embedding, wq, wk, wv, wo, w1, w2, w3, wcls }
    }

    /// The output classifier, which is the token embedding when shared.
    pub fn wcls(&self) -> &QMatrix<'a> {
        self.wcls.as_ref().unwrap_or(&self.embedding)
    }

    /// Number of weights (int8 values) in the model.
    pub fn params(&self) -> usize {
        let mats =
            [&self.embedding, &self.wq, &self.wk, &self.wv, &self.wo, &self.w1, &self.w2, &self.w3];
        mats.iter().map(|m| m.q.len()).sum::<usize>() + self.wcls.as_ref().map_or(0, |m| m.q.len())
    }

    /// Integer multiply-accumulates in the matrix multiplications of one
    /// forward pass (attention over the KV cache is not included).
    pub fn matmul_macs_per_token(&self) -> usize {
        let c = &self.config;
        let per_layer = 2 * c.dim * c.dim + 2 * c.kv_dim() * c.dim + 3 * c.hidden_dim * c.dim;
        c.n_layers * per_layer + c.vocab_size * c.dim
    }
}
