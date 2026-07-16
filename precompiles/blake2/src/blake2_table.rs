//! The `Blake2brTableSM` module defines the Blake2br Table State Machine.
//!
//! This state machine is responsible for calculating Blake2br XOR table rows.

/// The `Blake2brTableSM` struct represents the Blake2br Table State Machine.
pub struct Blake2brTableSM;

impl Blake2brTableSM {
    pub const TABLE_ID: usize = 128;

    /// Calculates the table row for the XOR tuple (a, b, a ^ b).
    ///
    /// # Arguments
    /// * `a` - The first input byte.
    /// * `b` - The second input byte.
    ///
    /// # Returns
    /// The calculated table row offset.
    pub const fn calculate_table_row(a: u8, b: u8) -> u32 {
        a as u32 + (b as u32) * 256
    }
}
