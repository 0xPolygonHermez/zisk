use crate::babyjubjub_constants::{BABYJUBJUB_CHUNKS, BABYJUBJUB_CHUNK_BITS};

#[derive(Clone, Debug)]
pub struct EquationConfig {
    pub chunks: usize,
    pub chunk_bits: usize,
    pub terms_by_clock: usize,
    pub comment_col: usize,
}

impl Default for EquationConfig {
    fn default() -> Self {
        Self {
            chunks: BABYJUBJUB_CHUNKS,
            chunk_bits: BABYJUBJUB_CHUNK_BITS,
            terms_by_clock: 2,
            comment_col: 30,
        }
    }
}
