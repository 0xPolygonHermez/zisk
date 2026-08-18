//! The `KeccakfChiTableSM` module defines the Keccakf χ-row S-box Table State Machine.
//!
//! This state machine is responsible for calculating χ-row S-box table rows.

use super::{CHI_BASE, CHI_SPAN, CHI_TABLE_SIZE, SLOT};

/// The `KeccakfChiTableSM` struct represents the Keccakf χ-row S-box Table State Machine.
pub struct KeccakfChiTableSM;

impl KeccakfChiTableSM {
    /// Must mirror KECCAKF_CHI_TABLE_ID in pil/opids.pil
    pub const TABLE_ID: usize = 126;

    /// Calculates the table row for one sliced χ-row lookup.
    ///
    /// # Arguments
    /// * `ta` - The five θ-outputs of instance A (each in [0,3]), indexed by x.
    /// * `tb` - The five θ-outputs of instance B (each in [0,3]), indexed by x.
    /// * `rc` - The ι round-constant bit of the χ-row (only y = 0 rows carry it).
    ///
    /// # Returns
    /// The table row index: rc·16⁵ + Σ_x (tA_x + 4·tB_x)·16ˣ.
    #[inline(always)]
    pub fn calculate_table_row(ta: &[u8; 5], tb: &[u8; 5], rc: bool) -> u32 {
        let mut row: u32 = 0;
        let mut x = 5;
        while x > 0 {
            x -= 1;
            debug_assert!(ta[x] < 4 && tb[x] < 4, "θ-output exceeds the [0,3] range");
            row = row * 16 + (ta[x] as u32 + 4 * tb[x] as u32);
        }
        if rc {
            row += 16u32.pow(5);
        }
        debug_assert!(row < CHI_TABLE_SIZE);
        row
    }

    /// Calculates the packed lookup INPUT of one sliced χ-row: the value the
    /// table's A column stores at `calculate_table_row`'s index, and the value
    /// committed in `sbox_acc` on narrow layouts.
    ///
    /// Only the narrow (lanes_per_row < 25) layouts commit this value; the wide
    /// layout packs the θ-outputs into the lookup expression directly.
    ///
    /// # Returns
    /// rc·28⁵ + Σ_x (tA_x + 8·tB_x)·28ˣ.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn calculate_table_input(ta: &[u8; 5], tb: &[u8; 5], rc: bool) -> u32 {
        let mut value: u32 = 0;
        let mut x = 5;
        while x > 0 {
            x -= 1;
            debug_assert!(ta[x] < 4 && tb[x] < 4, "θ-output exceeds the [0,3] range");
            value = value * CHI_BASE + (ta[x] as u32 + SLOT as u32 * tb[x] as u32);
        }
        if rc {
            value += CHI_SPAN;
        }
        value
    }
}
