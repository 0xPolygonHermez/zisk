use crate::arith_eq_constants::{ARITH_EQ_CHUNKS, ARITH_EQ_CHUNK_BITS};

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
            chunks: ARITH_EQ_CHUNKS,
            chunk_bits: ARITH_EQ_CHUNK_BITS,
            terms_by_clock: 2,
            comment_col: 30,
        }
    }
}
