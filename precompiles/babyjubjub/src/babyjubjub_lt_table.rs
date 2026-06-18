//! The `BabyJubJubLtTableSM` module defines the BabyJubJub "less-than" Table State Machine.
//!
//! It mirrors the arith_eq LT table but owns a distinct `TABLE_ID`, so the BabyJubJub AIR
//! can range-check that `x3 < p` and `y3 < p` (alias-free reduced coordinates) independently
//! of the shared arith_eq table.

/// The `BabyJubJubLtTableSM` struct represents the BabyJubJub Lt Table State Machine.
pub struct BabyJubJubLtTableSM;

impl BabyJubJubLtTableSM {
    pub const TABLE_ID: usize = 5003;

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
