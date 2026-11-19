//! Parses a model blob (see [`crate::checkpoint`]) without copying the
//! quantized weights; only the small `f32` arrays are copied.

use alloc::vec::Vec;

use crate::checkpoint::{Config, MAGIC, VERSION};
use crate::fixed;
use crate::formats::Format;
use crate::{read_f32, read_u32};

/// A quantized matrix of `rows` x `cols`, in groups of `gs` along each row,
/// each with a scale per unit (see [`crate::formats`]).
pub struct QMatrix<'a> {
    pub rows: usize,
    pub cols: usize,
    pub gs: usize,
    pub format: Format,
    /// The values' bytes: one per value, or one per two values for E2M1.
    pub q: &'a [u8],
    pub scales: Vec<f32>,
    /// The scales in fixed point (see [`crate::fixed`]): `scales[g] ≈
    /// fixed_scales[g] · 2^-row_shift[row]`.
    pub fixed_scales: Vec<i32>,
    pub row_shift: Vec<i32>,
}

impl QMatrix<'_> {
    /// Rows `[first, first + n)`, as a view sharing the weights.
    pub fn rows(&self, first: usize, n: usize) -> QMatrixView<'_> {
        QMatrixView {
            rows: n,
            cols: self.cols,
            gs: self.gs,
            format: self.format,
            q: &self.q
                [self.format.bytes(first * self.cols)..self.format.bytes((first + n) * self.cols)],
            scales: &self.scales[first * self.cols / self.gs..(first + n) * self.cols / self.gs],
            fixed_scales: &self.fixed_scales
                [first * self.cols / self.gs..(first + n) * self.cols / self.gs],
            row_shift: &self.row_shift[first..first + n],
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
    pub format: Format,
    pub q: &'a [u8],
    pub scales: &'a [f32],
    pub fixed_scales: &'a [i32],
    pub row_shift: &'a [i32],
}

/// The parsed model. Per-layer matrices are stacked: layer `l` of `wq` is
/// `wq.rows(l * dim, dim)`.
pub struct Model<'a> {
    pub config: Config,
    /// The format the activations are quantized to before each matrix
    /// multiplication.
    pub activations: Format,
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
    format: Format,
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

    fn align8(&mut self) {
        self.pos = self.pos.next_multiple_of(8);
    }

    fn qmatrix(&mut self, rows: usize, cols: usize, gs: usize) -> QMatrix<'a> {
        let (n, format) = (rows * cols, self.format);
        let q = &self.blob[self.pos..self.pos + format.bytes(n)];
        self.pos += format.bytes(n);
        self.align8();
        let scales = self.floats(n / gs);
        self.align8();
        assert!(cols <= fixed::MAX_LEN, "matrix rows of {cols} values are too long");
        let mut fixed_scales = alloc::vec![0; n / gs];
        let row_shift = scales
            .chunks_exact(cols / gs)
            .zip(fixed_scales.chunks_exact_mut(cols / gs))
            .map(|(s, f)| fixed::scales_to_fixed(s, f))
            .collect();
        QMatrix { rows, cols, gs, format, q, scales, fixed_scales, row_shift }
    }
}

impl<'a> Model<'a> {
    /// Parses a model blob. Panics if the blob is malformed.
    pub fn parse(blob: &'a [u8]) -> Self {
        let mut c = Cursor { blob, pos: 0, format: Format::Int8 };
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
        let format = |id| Format::from_id(id).expect("unknown format in the model blob");
        c.format = format(c.u32() as u32);
        let activations = format(c.u32() as u32);
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

        Model {
            config,
            activations,
            rms_att,
            rms_ffn,
            rms_final,
            embedding,
            wq,
            wk,
            wv,
            wo,
            w1,
            w2,
            w3,
            wcls,
        }
    }

    /// The output classifier, which is the token embedding when shared.
    pub fn wcls(&self) -> &QMatrix<'a> {
        self.wcls.as_ref().unwrap_or(&self.embedding)
    }

    /// The format of the weights.
    pub fn weights(&self) -> Format {
        self.wq.format
    }

    /// Number of weights in the model.
    pub fn params(&self) -> usize {
        let mats =
            [&self.embedding, &self.wq, &self.wk, &self.wv, &self.wo, &self.w1, &self.w2, &self.w3];
        let n = |m: &QMatrix| m.rows * m.cols;
        mats.iter().map(|m| n(m)).sum::<usize>() + self.wcls.as_ref().map_or(0, n)
    }

    /// Integer multiply-accumulates in the matrix multiplications of one
    /// forward pass (attention over the KV cache is not included).
    pub fn matmul_macs_per_token(&self) -> usize {
        let c = &self.config;
        let per_layer = 2 * c.dim * c.dim + 2 * c.kv_dim() * c.dim + 3 * c.hidden_dim * c.dim;
        c.n_layers * per_layer + c.vocab_size * c.dim
    }
}
