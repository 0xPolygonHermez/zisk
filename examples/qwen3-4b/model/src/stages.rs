//! Row authentication and stage execution for the Qwen3-4B tied embedding and head.
pub use crate::receipts::{DIM, Hash, LAYERS, VOCAB};

pub type Sha = fn(&[u8]) -> Hash;

/// Bytes of one BF16 row of the tied matrix.
pub const ROW_BYTES: usize = 2 * DIM as usize;

pub fn tagged(sha: Sha, domain: &[u8], parts: &[&[u8]]) -> Hash {
    let mut pre =
        Vec::with_capacity(8 + domain.len() + parts.iter().map(|p| p.len()).sum::<usize>());
    pre.extend_from_slice(b"PCI/v1/");
    pre.extend_from_slice(domain);
    pre.push(0);
    for p in parts {
        pre.extend_from_slice(p);
    }
    sha(&pre)
}

pub(crate) fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

pub(crate) fn hash_at(b: &[u8], at: usize) -> Hash {
    b[at..at + 32].try_into().unwrap()
}

/// Exact BF16 to FP32 bit widening; rejects nonfinite values.
pub(crate) fn widen_finite(bf16: &[u8], out: &mut Vec<u8>) -> Result<(), &'static str> {
    for p in bf16.chunks_exact(2) {
        let v = u16::from_le_bytes([p[0], p[1]]);
        if (v >> 7) & 0xff == 0xff {
            return Err("nonfinite weight");
        }
        out.extend_from_slice(&(u32::from(v) << 16).to_le_bytes());
    }
    Ok(())
}

pub mod rows {
    //! Fixed-depth binary Merkle tree over the tied matrix rows.
    //!
    //! ```text
    //! leaf(i)  = H("row",      i u32 || 5120 raw BF16 bytes)     i < 151936
    //! leaf(i)  = 32 zero bytes                                   151936 <= i < 2^18
    //! node     = H("row-node", left || right)
    //! root     = H("rows-root", VOCAB u32 || ROW_BYTES u32 || DEPTH u32 || top)
    //! ```
    use super::*;

    pub const DEPTH: u32 = 18;
    const _: () = assert!(VOCAB <= 1 << DEPTH);

    pub fn leaf(sha: Sha, index: u32, row: &[u8]) -> Hash {
        debug_assert_eq!(row.len(), ROW_BYTES);
        tagged(sha, b"row", &[&index.to_le_bytes(), row])
    }

    pub fn node(sha: Sha, left: &Hash, right: &Hash) -> Hash {
        tagged(sha, b"row-node", &[left, right])
    }

    pub fn root(sha: Sha, top: &Hash) -> Hash {
        tagged(
            sha,
            b"rows-root",
            &[&VOCAB.to_le_bytes(), &(ROW_BYTES as u32).to_le_bytes(), &DEPTH.to_le_bytes(), top],
        )
    }

    /// Climb from node `index` at `level` to the top using `siblings`
    /// (bottom-up, `DEPTH - level` of them) and wrap the result as a root.
    pub fn climb(sha: Sha, mut h: Hash, mut index: u32, level: u32, siblings: &[u8]) -> Hash {
        assert_eq!(siblings.len(), 32 * (DEPTH - level) as usize);
        for s in siblings.chunks_exact(32) {
            let s: &Hash = s.try_into().unwrap();
            h = if index & 1 == 0 { node(sha, &h, s) } else { node(sha, s, &h) };
            index >>= 1;
        }
        root(sha, &h)
    }
}

/// Weights root of the embedding child: the row commitment it opened.
pub fn embed_weights(sha: Sha, rows_root: &Hash) -> Hash {
    tagged(sha, b"embed-weights/rows-v1", &[rows_root])
}

/// Weights root of every head shard: SHA-256 of the raw final-norm bytes and
/// the row commitment.
pub fn head_weights(sha: Sha, norm_sha: &Hash, rows_root: &Hash) -> Hash {
    tagged(sha, b"head-weights/rows-v1", &[norm_sha, rows_root])
}

pub(crate) fn activation_root(
    sha: Sha,
    model: Hash,
    request: Hash,
    boundary: u32,
    positions: u32,
    values: &[u8],
) -> Result<Hash, &'static str> {
    let mut pre = crate::receipts::activation_prefix(model, request, boundary, positions)
        .map_err(|_| "activation shape")?;
    pre.extend_from_slice(values);
    Ok(sha(&pre))
}

pub mod embed {
    //! Record v1 (little-endian, no trailing bytes):
    //! ```text
    //! "PCIEMBD1" | model 32 | suite 32 | request 32 | T u32 | T token IDs u32
    //! | per position: row 5120 raw BF16 bytes | 18 siblings x 32
    //! ```
    //! The receipt is a `crate::receipts::Receipt` of role Embed: input is the
    //! consumed-token root of the T IDs, output the boundary-0 activation root
    //! of the exactly widened rows, weights [`embed_weights`] of the single
    //! row root every opening reproduced.
    use super::*;
    use crate::receipts::{Receipt, Role, consumed_root};

    pub const MAGIC: &[u8; 8] = b"PCIEMBD1";
    pub const FIXED: usize = 8 + 3 * 32 + 4;
    const OPENING: usize = ROW_BYTES + 32 * rows::DEPTH as usize;

    pub struct Outcome {
        pub receipt: Receipt,
        pub rows_root: Hash,
        pub digest: Hash,
        /// A0 as little-endian FP32 bytes.
        pub output: Vec<u8>,
    }

    pub fn execute(record: &[u8], sha: Sha) -> Result<Outcome, &'static str> {
        if record.len() < FIXED || &record[..8] != MAGIC {
            return Err("magic");
        }
        let (model, suite, request) =
            (hash_at(record, 8), hash_at(record, 40), hash_at(record, 72));
        let t = u32_at(record, 104);
        if !(1..=40_960).contains(&t) {
            return Err("positions");
        }
        let n = t as usize;
        if record.len() != FIXED + 4 * n + n * OPENING {
            return Err("record length");
        }
        let ids: Vec<u32> = (0..n).map(|i| u32_at(record, FIXED + 4 * i)).collect();
        if ids.iter().any(|&id| id >= VOCAB) {
            return Err("token id out of range");
        }
        let mut root: Option<Hash> = None;
        let mut a0 = Vec::with_capacity(n * DIM as usize * 4);
        let base = FIXED + 4 * n;
        for (i, &id) in ids.iter().enumerate() {
            let o = &record[base + i * OPENING..][..OPENING];
            let (row, siblings) = o.split_at(ROW_BYTES);
            let r = rows::climb(sha, rows::leaf(sha, id, row), id, 0, siblings);
            if *root.get_or_insert(r) != r {
                return Err("openings disagree on the row root");
            }
            widen_finite(row, &mut a0)?;
        }
        let rows_root = root.unwrap();
        let receipt = Receipt {
            role: Role::Embed,
            positions: t,
            first_prediction: 0,
            predictions: 0,
            input: consumed_root(&ids, &[]),
            output: activation_root(sha, model, request, 0, t, &a0)?,
            weights: embed_weights(sha, &rows_root),
        };
        let digest = receipt.digest(model, suite, request);
        Ok(Outcome { receipt, rows_root, digest, output: a0 })
    }
}

pub mod head {
    //! One aligned vocabulary shard of the full head.
    //!
    //! Record v1 (little-endian, no trailing bytes):
    //! ```text
    //! "PCIHEAD1" | model 32 | suite 32 | request 32 | shard_log2 u32 | shard u32
    //! | T u32 | R u32 | A36: T x 2560 FP32 bits | final norm 5120 raw BF16 bytes
    //! | rows [shard*2^s, min((shard+1)*2^s, VOCAB)) raw BF16 | (18 - s) siblings x 32
    //! ```
    //! For each of the last R positions the guest applies the final RMSNorm
    //! and BF16 projection. Nonfinite logits reject the record. The shard winner is the lowest ID among the largest values
    //! (`>` comparison, so +0 and -0 tie). The receipt binds the whole window.
    use super::*;
    pub const MAGIC: &[u8; 8] = b"PCIHEAD1";
    pub const FIXED: usize = 8 + 3 * 32 + 4 * 4;
    /// Shard sizes the suite admits.
    pub const MIN_LOG2: u32 = 4;
    pub const MAX_LOG2: u32 = rows::DEPTH;

    pub fn shard_count(log2: u32) -> u32 {
        VOCAB.div_ceil(1 << log2)
    }

    pub fn shard_rows(log2: u32, shard: u32) -> u32 {
        (1u32 << log2).min(VOCAB - (shard << log2))
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ShardReceipt {
        pub shard_log2: u32,
        pub shard: u32,
        pub positions: u32,
        pub first_prediction: u32,
        pub predictions: u32,
        /// Boundary-36 activation root.
        pub input: Hash,
        /// [`head_weights`].
        pub weights: Hash,
        /// Per prediction: (winning token ID in this shard, its logit bits).
        pub best: Vec<(u32, u32)>,
    }

    impl ShardReceipt {
        pub fn digest(&self, sha: Sha, model: Hash, suite: Hash, request: Hash) -> Hash {
            let mut fields = Vec::with_capacity(5 * 4 + 64 + 8 * self.best.len());
            for v in [
                self.shard_log2,
                self.shard,
                self.positions,
                self.first_prediction,
                self.predictions,
            ] {
                fields.extend(v.to_le_bytes());
            }
            fields.extend(self.input);
            fields.extend(self.weights);
            fields.extend((self.best.len() as u32).to_le_bytes());
            for (id, bits) in &self.best {
                fields.extend(id.to_le_bytes());
                fields.extend(bits.to_le_bytes());
            }
            tagged(sha, b"head-shard/v1", &[&model, &suite, &request, &fields])
        }
    }

    pub struct Outcome {
        pub receipt: ShardReceipt,
        pub rows_root: Hash,
        pub digest: Hash,
        /// R x shard rows logits, for diagnostics.
        pub logits: Vec<f32>,
    }

    /// A batched projection kernel: `x` holds `positions x 2560` FP32 bit
    /// patterns, `out` receives `positions x rows` logits (`out[t * rows + r]`).
    /// Only bit-identical kernels reproduce the pinned reference receipt.
    pub type Kernel<'a> = &'a mut dyn FnMut(&[u32], &[u8], &mut [f32]);
    pub fn execute_with(record: &[u8], sha: Sha, kernel: Kernel) -> Result<Outcome, &'static str> {
        if record.len() < FIXED || &record[..8] != MAGIC {
            return Err("magic");
        }
        let (model, suite, request) =
            (hash_at(record, 8), hash_at(record, 40), hash_at(record, 72));
        let (log2, shard, t, r) =
            (u32_at(record, 104), u32_at(record, 108), u32_at(record, 112), u32_at(record, 116));
        if !(MIN_LOG2..=MAX_LOG2).contains(&log2) || shard >= shard_count(log2) {
            return Err("shard");
        }
        if !(1..=40_960).contains(&t) || r == 0 || r > t {
            return Err("positions");
        }
        let rows = shard_rows(log2, shard) as usize;
        let act = t as usize * DIM as usize * 4;
        let sib = 32 * (rows::DEPTH - log2) as usize;
        if record.len() != FIXED + act + ROW_BYTES + rows * ROW_BYTES + sib {
            return Err("record length");
        }
        let x = &record[FIXED..FIXED + act];
        let norm = &record[FIXED + act..][..ROW_BYTES];
        let matrix = &record[FIXED + act + ROW_BYTES..][..rows * ROW_BYTES];
        let siblings = &record[FIXED + act + ROW_BYTES + rows * ROW_BYTES..];

        // Authenticate the shard: hash its rows, pad to 2^s leaves, climb.
        let first_id = shard << log2;
        let mut level: Vec<Hash> = matrix
            .chunks_exact(ROW_BYTES)
            .enumerate()
            .map(|(i, row)| rows::leaf(sha, first_id + i as u32, row))
            .collect();
        // Leaves past the vocabulary are the zero padding leaf.
        level.resize(1 << log2, [0u8; 32]);
        for _ in 0..log2 {
            level = level.chunks_exact(2).map(|p| rows::node(sha, &p[0], &p[1])).collect();
        }
        let rows_root = rows::climb(sha, level[0], shard, log2, siblings);
        let weights = head_weights(sha, &sha(norm), &rows_root);
        let input = activation_root(sha, model, request, LAYERS as u32, t, x)?;

        // Authenticate and validate all T positions. Only the last R enter the final norm.
        if x.chunks_exact(4).any(|v| (u32_at(v, 0) >> 23) & 0xff == 0xff) {
            return Err("nonfinite input activation");
        }
        let width = DIM as usize;
        let n = r as usize * width;
        let norm_weights: Vec<f32> = crate::layer::widen(norm).collect();
        if norm_weights.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite weight");
        }
        let mut xn = Vec::with_capacity(n);
        let mut normalized = vec![0.0; width];
        for row in x[x.len() - 4 * n..].chunks_exact(4 * width) {
            let input: Vec<f32> =
                row.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
            crate::decoder::bf16_rmsnorm(&mut normalized, &input, &norm_weights, 1e-6);
            xn.extend(normalized.iter().map(|v| v.to_bits()));
        }
        let mut logits = vec![0.0f32; r as usize * rows];
        kernel(&xn, matrix, &mut logits);
        if logits.len() != r as usize * rows || logits.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite or misshapen logits");
        }
        let best = logits
            .chunks_exact(rows)
            .map(|l| {
                let mut b = 0;
                for (i, v) in l.iter().enumerate() {
                    if *v > l[b] {
                        b = i;
                    }
                }
                (first_id + b as u32, l[b].to_bits())
            })
            .collect();
        let receipt = ShardReceipt {
            shard_log2: log2,
            shard,
            positions: t,
            first_prediction: t - r,
            predictions: r,
            input,
            weights,
            best,
        };
        let digest = receipt.digest(sha, model, suite, request);
        Ok(Outcome { receipt, rows_root, digest, logits })
    }
}
