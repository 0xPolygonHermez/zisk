//! The `KeccakfXor5TableSM` module defines the Keccakf xor5 Table State Machine.
//!
//! This state machine is responsible for calculating xor5 normalization table rows.

use super::{XOR5_BATCH, XOR5_TABLE_SIZE, XOR5_VALUES};

/// The `KeccakfXor5TableSM` struct represents the Keccakf xor5 Table State Machine.
pub struct KeccakfXor5TableSM;

impl KeccakfXor5TableSM {
    /// Must mirror KECCAKF_XOR5_TABLE_ID in pil/opids.pil
    pub const TABLE_ID: usize = 127;

    /// Calculates the table row for one batch of three column sums.
    ///
    /// # Arguments
    /// * `sums` - Three (sA, sB) column-sum pairs, each component in [0,5].
    ///
    /// # Returns
    /// The table row index: Σ_k (sA_k + 6·sB_k)·36ᵏ.
    #[inline(always)]
    pub fn calculate_table_row(sums: &[(u8, u8); XOR5_BATCH]) -> u32 {
        let mut row: u32 = 0;
        let mut k = XOR5_BATCH;
        while k > 0 {
            k -= 1;
            let (sa, sb) = sums[k];
            debug_assert!(sa < 6 && sb < 6, "column sum exceeds the [0,5] range");
            row = row * XOR5_VALUES + (sa as u32 + 6 * sb as u32);
        }
        debug_assert!(row < XOR5_TABLE_SIZE);
        row
    }
}
