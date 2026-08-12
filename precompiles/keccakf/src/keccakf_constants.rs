use zisk_core::P2_23;

pub(crate) const LANES: usize = 25;
pub(crate) const LANE_BITS: usize = 64;
pub(crate) const WIDTH: usize = LANES * LANE_BITS;

/// Number of state lanes held by each trace row.
pub(crate) const LANES_PER_ROW: usize = 5;
pub(crate) const ROWS_PER_STATE: usize = LANES.div_ceil(LANES_PER_ROW);
pub(crate) const LANES_LAST_ROW: usize = LANES - (ROWS_PER_STATE - 1) * LANES_PER_ROW;
pub(crate) const BITS_PER_ROW: usize = LANES_PER_ROW * LANE_BITS;
pub(crate) const BITS_LAST_ROW: usize = LANES_LAST_ROW * LANE_BITS;

pub(crate) const ROUNDS: usize = 24;
pub(crate) const CLOCKS: usize = (1 + ROUNDS) * ROWS_PER_STATE;

/// The maximum value that any expression during keccakf computation can get
const MAX_EXPR_VALUE: u32 = 144;

pub(crate) const BASE: u32 = MAX_EXPR_VALUE + 1;

pub(crate) const TABLE_MAX_CHUNKS: usize = calculate_chunk_size() as usize;
pub(crate) const TABLE_SIZE: u32 = BASE.pow(calculate_chunk_size());
pub(crate) const CHUNKS_PER_ROW: usize = BITS_PER_ROW.div_ceil(TABLE_MAX_CHUNKS);

pub(crate) const POWS_BASE: [u32; TABLE_MAX_CHUNKS] = {
    let mut pow = [1u32; TABLE_MAX_CHUNKS];
    let mut i = 1;
    while i < TABLE_MAX_CHUNKS {
        pow[i] = pow[i - 1] * BASE;
        i += 1;
    }
    pow
};

const fn calculate_chunk_size() -> u32 {
    let mut chunks = 1;
    while (BASE.pow(chunks + 1) as u64) < P2_23 {
        chunks += 1;
    }
    chunks
}
