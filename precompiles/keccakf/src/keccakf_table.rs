//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for calculating Keccakf table rows.

use crate::{GROUP_BY, MAX_VALUE};

/// The `KeccakfTableSM` struct represents the Keccakf Table State Machine.
pub struct KeccakfTableSM;

impl KeccakfTableSM {
    pub const TABLE_ID: usize = 126;

    pub const BASE: u32 = MAX_VALUE + 1;

    pub const MAX: u32 = Self::calculate_max();

    const fn calculate_max() -> u32 {
        let mut max = 0;
        let mut i = 0;
        while i < GROUP_BY {
            max += MAX_VALUE * Self::BASE.pow(i as u32);
            i += 1;
        }
        max
    }

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `a` - The input value used to calculate the table row.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn calculate_table_row(a: u32) -> u32 {
        debug_assert!(a <= MAX_VALUE, "Operand 'a' exceeds maximum value");

        // let mut ajs = [0u32; GROUP_BY];
        // for i in 0..GROUP_BY {
        //     ajs[i] = (a / Self::BASE.pow(i as u32)) % Self::BASE;
        // }

        a
    }
}
