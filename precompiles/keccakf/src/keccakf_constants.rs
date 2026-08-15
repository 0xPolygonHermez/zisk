pub(crate) const WIDTH: usize = 1600;

pub(crate) const ROUNDS: usize = 24;
pub(crate) const CLOCKS: usize = 1 + ROUNDS;

/// The χ-row S-box lookup packs the five θ-outputs of a χ-row (values in [0,11])
/// in base 12, and adds the ι round-constant bit on top:
///     row = rc·12⁵ + Σ_x t_x·12ˣ
pub(crate) const SBOX_BASE: u32 = 12;
pub(crate) const SBOX_INPUTS: usize = 5;
pub(crate) const SBOX_SPAN: u32 = SBOX_BASE.pow(SBOX_INPUTS as u32);
pub(crate) const TABLE_SIZE: u32 = 2 * SBOX_SPAN;
