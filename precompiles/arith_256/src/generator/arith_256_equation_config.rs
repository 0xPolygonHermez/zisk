#[derive(Clone, Debug)]
pub struct Arith256EquationConfig {
    pub chunks: usize,
    pub chunk_bits: usize,
    pub terms_by_clock: usize,
    pub comment_col: usize,
}

impl Default for Arith256EquationConfig {
    fn default() -> Self {
        Self { chunks: 16, chunk_bits: 16, terms_by_clock: 2, comment_col: 30 }
    }
}
