//! The `KeccakfTableSM` module defines the Keccakf Table State Machine.
//!
//! This state machine is responsible for calculating Keccakf table rows.

use super::TABLE_SIZE;

/// The `KeccakfTableSM` struct represents the Keccakf Table State Machine.
pub struct KeccakfTableSM;

impl KeccakfTableSM {
    pub const TABLE_ID: usize = 126;

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `a` - The input value used to calculate the table row.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub const fn calculate_table_row(a: u32) -> u32 {
        debug_assert!(a < TABLE_SIZE, "Operand 'a' exceeds maximum value");
        a
    }
}
