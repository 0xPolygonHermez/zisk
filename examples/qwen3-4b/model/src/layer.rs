//! Record v1 (little-endian, no trailing bytes):
//! ```text
//! "PCILAYR1" | model 32 | suite 32 | request 32 | layer u32 | positions u32
//! | profile header 64 (must equal crate::receipts::layer_profile(positions, R), 1 <= R <= positions)
//! | A_layer: positions x 2560 FP32 bits
//! | 11 raw BF16 checkpoint tensors in checkpoint name order
//! ```
use crate::receipts::{
    DIM, Hash, LAYERS, Receipt, Role, activation_prefix, layer_profile, weights_root,
};
const HEADER_BYTES: usize = 64;

pub const MAGIC: &[u8; 8] = b"PCILAYR1";
pub const FIXED: usize = 8 + 3 * 32 + 4 + 4 + HEADER_BYTES;
pub type Sha = fn(&[u8]) -> Hash;

/// BF16 element counts in checkpoint name order.
pub const TENSOR_LEN: [usize; 11] = [
    2560,
    2560 * 9728,
    9728 * 2560,
    9728 * 2560,
    2560,
    128,
    1024 * 2560,
    2560 * 4096,
    128,
    4096 * 2560,
    1024 * 2560,
];
/// (Tensor index, rows, columns) for q, k, v, o, gate, up and down.
const PROJECTIONS: [(usize, usize, usize); 7] = [
    (9, 4096, 2560),
    (6, 1024, 2560),
    (10, 1024, 2560),
    (7, 2560, 4096),
    (2, 9728, 2560),
    (3, 9728, 2560),
    (1, 2560, 9728),
];
const INPUT_LN: usize = 0;
const POST_LN: usize = 4;
const K_NORM: usize = 5;
const Q_NORM: usize = 8;

pub struct Outcome {
    pub receipt: Receipt,
    pub output: Vec<f32>,
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// Exact BF16 to FP32 widening.
pub fn widen(bf16: &[u8]) -> impl Iterator<Item = f32> + '_ {
    bf16.chunks_exact(2).map(|p| f32::from_bits(u32::from(u16::from_le_bytes([p[0], p[1]])) << 16))
}

fn output_root(
    sha: Sha,
    model: Hash,
    request: Hash,
    boundary: u32,
    positions: u32,
    values: &[f32],
) -> Result<Hash, &'static str> {
    let mut pre =
        activation_prefix(model, request, boundary, positions).map_err(|_| "activation shape")?;
    pre.reserve_exact(values.len() * 4);
    for value in values {
        pre.extend_from_slice(&value.to_le_bytes());
    }
    Ok(sha(&pre))
}
/// Receipt digest the guest commits for a record.
pub fn digest_of(record: &[u8], receipt: &Receipt) -> Hash {
    let id = |at: usize| -> Hash { record[at..at + 32].try_into().unwrap() };
    receipt.digest(id(8), id(40), id(72))
}
pub type ProjectBatch<'a> = &'a mut dyn FnMut(&[u32], usize, &[&[u8]], &mut [&mut [f32]]);
pub fn execute_batched(
    record: &[u8],
    sha: Sha,
    kernel: ProjectBatch,
) -> Result<Outcome, &'static str> {
    if record.len() < FIXED || &record[..8] != MAGIC {
        return Err("magic");
    }
    let id = |at: usize| -> Hash { record[at..at + 32].try_into().unwrap() };
    let (model, request) = (id(8), id(72));
    let index = u32_at(record, 104);
    let positions = u32_at(record, 108);
    if index as usize >= LAYERS || !(1..=40_960).contains(&positions) {
        return Err("layer index or positions");
    }
    // Header n_out carries R: the prompt is the first T - R + 1 positions.
    let predictions = u32_at(record, 112 + 44);
    if !(1..=positions).contains(&predictions) {
        return Err("prediction window");
    }
    let profile = layer_profile(positions, predictions);
    if record[112..FIXED] != profile {
        return Err("arithmetic profile");
    }
    let t = positions as usize;
    let act_bytes = t * DIM as usize * 4;
    let expected = FIXED + act_bytes + TENSOR_LEN.iter().map(|n| 2 * n).sum::<usize>();
    if record.len() != expected {
        return Err("record length");
    }
    let x_bytes = &record[FIXED..FIXED + act_bytes];
    let mut tensors = [&record[..0]; 11];
    let mut at = FIXED + act_bytes;
    for (slot, n) in tensors.iter_mut().zip(TENSOR_LEN) {
        *slot = &record[at..at + 2 * n];
        at += 2 * n;
    }

    let digests: Vec<Hash> = tensors.iter().map(|t| sha(t)).collect();
    let role = Role::Layer(index);
    let weights = weights_root(role, &digests);
    let input = crate::stages::activation_root(sha, model, request, index, positions, x_bytes)?;

    let x: Vec<f32> =
        x_bytes.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
    if x.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite input activation");
    }
    let norm = [INPUT_LN, Q_NORM, K_NORM, POST_LN].map(|i| widen(tensors[i]).collect());
    let mut claim = |group: usize, x: &[f32], outs: &mut [&mut [f32]]| {
        let members = crate::decoder::BATCH_GROUPS[group];
        assert_eq!(outs.len(), members.len(), "projection group");
        let cols = PROJECTIONS[members[0]].2;
        assert_eq!(x.len(), t * cols, "projection input shape");
        let mut matrices = [&record[..0]; 3];
        for (k, &m) in members.iter().enumerate() {
            let (slot, rows, c) = PROJECTIONS[m];
            assert_eq!((c, outs[k].len()), (cols, t * rows), "projection shape");
            matrices[k] = tensors[slot];
        }
        // SAFETY: f32 to u32 reinterpretation of the same memory.
        let bits = unsafe { std::slice::from_raw_parts(x.as_ptr().cast::<u32>(), x.len()) };
        kernel(bits, cols, &matrices[..members.len()], outs);
    };
    let output = crate::decoder::run_layer_batched(
        x,
        &norm,
        (positions - predictions + 1) as usize,
        &mut claim,
    );
    if output.len() != t * DIM as usize || output.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite or misshapen output");
    }
    let out_root = output_root(sha, model, request, index + 1, positions, &output)?;
    Ok(Outcome {
        receipt: Receipt {
            role,
            positions,
            first_prediction: positions - predictions,
            predictions,
            input,
            output: out_root,
            weights,
        },
        output,
    })
}
