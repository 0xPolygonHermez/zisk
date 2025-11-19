use zisk_core::P2_23;

pub(crate) const WIDTH: usize = 1600;

pub(crate) const ROWS_BY_KECCAKF: usize = 25;

/// The maximum value that any expression during keccakf computation can get
/// Obtained from `keccakf_expr_generator.rs`
const MAX_EXPR_VALUE: u32 = 144;

pub(crate) const BASE: u32 = MAX_EXPR_VALUE + 1;

pub(crate) const TABLE_CHUNK_SIZE: usize = calculate_chunk_size() as usize;
pub(crate) const TABLE_SIZE: u32 = BASE.pow(calculate_chunk_size());

const fn calculate_chunk_size() -> u32 {
    let mut chunks = 1;
    while (BASE.pow(chunks + 1) as u64) < P2_23 {
        chunks += 1;
    }
    chunks
}
