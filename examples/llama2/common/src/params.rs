//! The generation request, sent to the guest as one raw input slice.
//!
//! Layout (little-endian): `u32 max_new`, `f32 temperature`, `u64 seed`,
//! `u32 n_prompt`, `u32 prompt[n_prompt]`.

use alloc::vec::Vec;

use crate::{read_f32, read_u32};

#[derive(Clone, Debug, PartialEq)]
pub struct GenParams {
    /// Prompt token ids, starting with BOS.
    pub prompt: Vec<u32>,
    /// Maximum number of tokens to generate after the prompt.
    pub max_new: u32,
    /// 0 for greedy decoding (always the most likely token).
    pub temperature: f32,
    /// Seed of the sampler's random generator (unused when greedy).
    pub seed: u64,
}

impl GenParams {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(20 + 4 * self.prompt.len());
        b.extend_from_slice(&self.max_new.to_le_bytes());
        b.extend_from_slice(&self.temperature.to_le_bytes());
        b.extend_from_slice(&self.seed.to_le_bytes());
        b.extend_from_slice(&(self.prompt.len() as u32).to_le_bytes());
        for t in &self.prompt {
            b.extend_from_slice(&t.to_le_bytes());
        }
        b
    }

    /// Panics if `b` is malformed.
    pub fn from_bytes(b: &[u8]) -> Self {
        let max_new = read_u32(b, 0);
        let temperature = read_f32(b, 4);
        let seed = u64::from_le_bytes(b[8..16].try_into().unwrap());
        let n = read_u32(b, 16) as usize;
        assert_eq!(b.len(), 20 + 4 * n, "malformed generation parameters");
        let prompt = (0..n).map(|i| read_u32(b, 20 + 4 * i)).collect();
        GenParams { prompt, max_new, temperature, seed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_bytes() {
        let p =
            GenParams { prompt: Vec::from([1, 403, 9038]), max_new: 7, temperature: 0.8, seed: 99 };
        assert_eq!(GenParams::from_bytes(&p.to_bytes()), p);
    }
}
