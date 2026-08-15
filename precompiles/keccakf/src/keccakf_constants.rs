pub(crate) const LANES: usize = 25;
pub(crate) const LANE_BITS: usize = 64;
pub(crate) const WIDTH: usize = LANES * LANE_BITS;

/// Number of state lanes held by each trace row.
pub(crate) const LANES_PER_ROW: usize = 25;
pub(crate) const ROWS_PER_STATE: usize = LANES / LANES_PER_ROW;
pub(crate) const BITS_PER_ROW: usize = LANES_PER_ROW * LANE_BITS;

pub(crate) const ROUNDS: usize = 24;
pub(crate) const CLOCKS: usize = (1 + ROUNDS) * ROWS_PER_STATE;

/// The χ-row S-box lookup packs the five θ-outputs of a χ-row (values in [0,11])
/// in base 12, and adds the ι round-constant bit on top:
///     row = rc·12⁵ + Σ_x t_x·12ˣ
pub(crate) const SBOX_BASE: u32 = 12;
pub(crate) const SBOX_INPUTS: usize = 5;
pub(crate) const SBOX_SPAN: u32 = SBOX_BASE.pow(SBOX_INPUTS as u32);
pub(crate) const TABLE_SIZE: u32 = 2 * SBOX_SPAN;
