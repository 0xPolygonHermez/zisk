//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for calculating Keccakf table rows.

use super::{SBOX_BASE, SBOX_SPAN, TABLE_SIZE};

/// The `KeccakfTableSM` struct represents the Keccakf Table State Machine.
pub struct KeccakfTableSM;

impl KeccakfTableSM {
    pub const TABLE_ID: usize = 126;

    /// Calculates the table row for one χ-row S-box lookup.
    ///
    /// # Arguments
    /// * `t` - The five θ-outputs of the χ-row (each in [0,11]), indexed by x.
    /// * `rc` - The ι round-constant bit of the χ-row (only y = 0 rows carry it).
    ///
    /// # Returns
    /// The table row index: rc·12⁵ + Σ_x t_x·12ˣ.
    #[inline(always)]
    pub fn calculate_table_row(t: &[u8; 5], rc: bool) -> u32 {
        let mut row: u32 = 0;
        for x in (0..5).rev() {
            debug_assert!(t[x] < SBOX_BASE as u8, "θ-output exceeds the base-12 range");
            row = row * SBOX_BASE + t[x] as u32;
        }
        if rc {
            row += SBOX_SPAN;
        }
        debug_assert!(row < TABLE_SIZE);
        row
    }
}
