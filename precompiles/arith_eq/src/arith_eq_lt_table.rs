//! The `ArithEqLtTableSM` module defines the ArithEqLt Table State Machine.
//!
//! This state machine is responsible for calculating ArithEqLt table rows.

/// The `ArithEqLtTableSM` struct represents the ArithEqLt Table State Machine.
pub struct ArithEqLtTableSM;

impl ArithEqLtTableSM {
    pub const TABLE_ID: usize = 5002;

    /// Calculates the table row offset based on the provided parameters.
    ///
    /// # Arguments
    /// * `prev_lt` - If previous current chunk of a is less than b, false at beginning
    /// * `lt` - If current chunk of a is less than b
    /// * `delta` - Difference between to values to compare (a - b)
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn calculate_table_row(prev_lt: bool, lt: bool, delta: i64) -> usize {
        // 0 0..MAX16
        // 1 -1..MIN16
        // 2 1..MAX16
        // 3 0..MIN16

        let index = match 2 * prev_lt as u8 + lt as u8 {
            0 => delta as usize,
            1 => 0x10000 + (-delta - 1) as usize,
            2 => 0x20000 + (delta - 1) as usize,
            3 => 0x30000 + (-delta) as usize,
            _ => panic!("Invalid range type"),
        };
        if index > 0x3FFFF {
            panic!("Invalid index:{index} prev_lt:{prev_lt} lt:{lt} delta:{delta}");
        }
        index
    }
}
