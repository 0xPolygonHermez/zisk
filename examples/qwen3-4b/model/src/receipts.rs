use sha2::{Digest, Sha256};
pub type Hash = [u8; 32];
pub const VOCAB: u32 = 151936;
pub const LAYERS: usize = 36;
pub const DIM: u32 = 2560;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Embed,
    Layer(u32),
    Head,
}
pub fn layer_profile(positions: u32, predictions: u32) -> [u8; 64] {
    let fields = [
        1,
        DIM,
        9728,
        32,
        8,
        128,
        VOCAB,
        positions,
        5_000_000f32.to_bits(),
        1e-6f32.to_bits(),
        32, // qwen_common::BF16_SERVING
        predictions,
    ];
    let mut out = [0; 64];
    for (o, f) in out.chunks_exact_mut(4).zip(fields) {
        o.copy_from_slice(&f.to_le_bytes());
    }
    out
}
pub fn weights_root(role: Role, tensors: &[Hash]) -> Hash {
    let (kind, index) = role.code();
    let mut h = start(b"stage-weights");
    h.update([kind]);
    h.update(index.to_le_bytes());
    h.update((tensors.len() as u32).to_le_bytes());
    for t in tensors {
        h.update(t);
    }
    h.finalize().into()
}
fn start(domain: &[u8]) -> Sha256 {
    let mut h = Sha256::new();
    h.update(b"PCI/v1/");
    h.update(domain);
    h.update([0]);
    h
}

fn ids(domain: &[u8], first: &[u32], second: &[u32]) -> Hash {
    let mut h = start(domain);
    h.update((first.len() as u64 + second.len() as u64).to_le_bytes());
    for id in first.iter().chain(second) {
        h.update(id.to_le_bytes());
    }
    h.finalize().into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub role: Role,
    pub positions: u32,
    /// For layers and the head these specify the prediction window. Zero for the embedding.
    pub first_prediction: u32,
    pub predictions: u32,
    /// Embedding consumes a token root; layers and head consume activation roots.
    pub input: Hash,
    /// Head produces a reply root; embedding and layers produce activation roots.
    pub output: Hash,
    /// [`weights_root`] of the tensors the guest hashed and consumed.
    pub weights: Hash,
}

impl Role {
    fn code(self) -> (u8, u32) {
        match self {
            Role::Embed => (0, 0),
            Role::Layer(i) => (1, i),
            Role::Head => (2, 0),
        }
    }
}

impl Receipt {
    pub fn digest(&self, model: Hash, suite: Hash, request: Hash) -> Hash {
        let mut h = start(b"child");
        h.update(model);
        h.update(suite);
        h.update(request);
        let (kind, index) = self.role.code();
        h.update([kind]);
        for v in [index, self.positions, self.first_prediction, self.predictions] {
            h.update(v.to_le_bytes());
        }
        h.update(self.input);
        h.update(self.output);
        h.update(self.weights);
        h.finalize().into()
    }
}
/// Bytes preceding the raw little-endian FP32 values in an activation root's
/// SHA-256 preimage. Guests hash `prefix || values` with the accelerated hash.
pub fn activation_prefix(
    model: Hash,
    request: Hash,
    boundary: u32,
    positions: u32,
) -> Result<Vec<u8>, &'static str> {
    if boundary > LAYERS as u32 || !(1..=40_960).contains(&positions) {
        return Err("activation shape");
    }
    let mut out = b"PCI/v1/activation\0".to_vec();
    out.extend(model);
    out.extend(request);
    for v in [boundary, positions, DIM] {
        out.extend(v.to_le_bytes());
    }
    Ok(out)
}

pub fn consumed_root(prompt: &[u32], feedback: &[u32]) -> Hash {
    ids(b"consumed", prompt, feedback)
}
