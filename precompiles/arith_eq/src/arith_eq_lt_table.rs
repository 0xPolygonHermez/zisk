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
    /// * `prev_lt` - Whether the previous chunk of `a` is less than `b`; `false` at the start
    /// * `lt` - Whether the current chunk of `a` is less than `b`
    /// * `delta` - Difference between the two values to compare (`a - b`)
    /// * `clock` - The clock position within the cycle: 0 = middle, 1 = first, 2 = last
    ///
    /// # Returns
    /// The calculated table row offset.
    pub fn calculate_table_row(prev_lt: bool, lt: bool, delta: i64, clock: u8) -> usize {
        // 0 0..MAX16
        // 1 -1..MIN16
        // 2 1..MAX16
        // 3 0..MIN16
        // 4 0..MAX16
        // 5 -1..MIN16
        // 6 0..MAX16
        // 7 -1..MIN16
        // 9 -1..MIN16
        // 11 0..MIN16

        let index = match clock * 4 + 2 * prev_lt as u8 + lt as u8 {
            // middle clocks
            0 => delta as usize,
            1 => 0x10000 + (-delta - 1) as usize,
            2 => 0x20000 + (delta - 1) as usize,
            3 => 0x30000 + (-delta) as usize,
            // first clock
            4 => 0x40000 + delta as usize,
            5 => 0x50000 + (-delta - 1) as usize,
            6 => 0x60000 + delta as usize,
            7 => 0x70000 + (-delta - 1) as usize,
            // last clock
            9 => 0x80000 + (-delta - 1) as usize,
            11 => 0x90000 + (-delta) as usize,
            _ => panic!("Invalid range type for clock:{clock} prev_lt:{prev_lt} lt:{lt}"),
        };
        if index > 0x9FFFF {
            panic!("Invalid index:{index} prev_lt:{prev_lt} lt:{lt} delta:{delta}");
        }
        index
    }
}
