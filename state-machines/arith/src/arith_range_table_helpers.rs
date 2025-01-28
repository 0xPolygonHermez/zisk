//! The `ArithRangeTableHelpers` and `ArithRangeTableInputs` modules define utilities and data
//! structures for managing and validating arithmetic range tables.
//!
//! ## Key Features
//!
//! ### `ArithRangeTableHelpers`
//! - Provides utilities for working with arithmetic range tables, including:
//!   - Translating range indices to human-readable names.
//!   - Calculating row indices for range and carry checks based on input values.
//!   - Ensures values adhere to their defined ranges, distinguishing between full, positive, and
//!     negative ranges.
//!
//! ### `ArithRangeTableInputs`
//! - Maintains and manages multiplicity data for range checks, including:
//!   - Tracking the frequency of range and carry checks.
//!   - Efficiently handling large tables using overflow storage for high-frequency rows.
//!   - Supports merging multiple `ArithRangeTableInputs` instances to aggregate data.
//! - Implements iterators to traverse all rows with non-zero multiplicity, enabling efficient
//!   processing.
//!
//! ## Key Components
//!
//! - **Range Definitions**: Defines constants (`FULL`, `POS`, `NEG`) and preconfigured range
//!   patterns for validating inputs.
//! - **Row Calculations**: Functions like `get_row_chunk_range_check` and
//!   `get_row_carry_range_check` calculate table rows based on inputs and ensure they comply with
//!   range constraints.
//! - **Multiplicity Tracking**: The `ArithRangeTableInputs` struct manages row-specific
//!   multiplicity data, supporting both direct and overflow storage for high-frequency updates.
//! - **Iterators**: Enable sequential access to multiplicity data, including rows with overflow
//!   values.
//!
//! These modules are critical for verifying range constraints in arithmetic operations, ensuring
//! correctness in high-assurance applications such as cryptographic proofs and hardware
//! simulations.

use std::collections::HashMap;

const ROWS: usize = 1 << 22;
const FULL: u8 = 0x00;
const POS: u8 = 0x01;
const NEG: u8 = 0x02;

/// The `ArithRangeTableHelpers` struct provides utility functions for working with
/// range tables, including converting range indices to names, calculating row indices,
/// and validating input ranges.
pub struct ArithRangeTableHelpers;

// Predefined range types for each range index.
const RANGES: [u8; 43] = [
    FULL, FULL, FULL, POS, POS, POS, NEG, NEG, NEG, FULL, FULL, FULL, FULL, FULL, FULL, FULL, FULL,
    FULL, POS, NEG, FULL, POS, NEG, FULL, POS, NEG, FULL, FULL, FULL, FULL, FULL, FULL, FULL, FULL,
    FULL, FULL, FULL, POS, POS, POS, NEG, NEG, NEG,
];

// Offset values corresponding to each range index.
const OFFSETS: [usize; 43] = [
    0, 2, 4, 50, 51, 52, 59, 60, 61, 6, 8, 10, 12, 14, 16, 18, 20, 22, 53, 62, 24, 54, 63, 26, 55,
    64, 28, 30, 32, 34, 36, 38, 40, 42, 44, 46, 48, 56, 57, 58, 65, 66, 67,
];

impl ArithRangeTableHelpers {
    /// Returns a human-readable name for a given range index.
    ///
    /// # Arguments
    /// * `range_index` - The index of the range whose name is needed.
    ///
    /// # Returns
    /// A string slice representing the name of the range.
    pub fn get_range_name(range_index: u8) -> &'static str {
        match range_index {
            0 => "F  F  F  F",
            1 => "F  F  +  F",
            2 => "F  F  -  F",
            3 => "+  F  F  F",
            4 => "+  F  +  F",
            5 => "+  F  -  F",
            6 => "-  F  F  F",
            7 => "-  F  +  F",
            8 => "-  F  -  F",
            9 => "F  F  F  +",
            10 => "F  F  F  -",
            11 => "F  +  F  F",
            12 => "F  +  F  +",
            13 => "F  +  F  -",
            14 => "F  -  F  F",
            15 => "F  -  F  +",
            16 => "F  -  F  -",
            _ => panic!("Invalid range index"),
        }
    }

    /// Calculates the row index for a chunk range check based on the range ID and value.
    ///
    /// # Arguments
    /// * `range_index` - The index of the range being checked.
    /// * `value` - The value being validated against the range.
    ///
    /// # Returns
    /// The calculated row index for the range table.
    pub fn get_row_chunk_range_check(range_index: u8, value: u64) -> usize {
        // F F F + + + - - - F F F F F F F F F + - F + - F + - F F F F F F F F F F F + + + - - -
        let range_type = RANGES[range_index as usize];
        assert!(range_index < 43);
        assert!(value >= if range_type == NEG { 0x8000 } else { 0 });
        assert!(
            value
                <= match range_type {
                    FULL => 0xFFFF,
                    POS => 0x7FFF,
                    NEG => 0xFFFF,
                    _ => panic!("Invalid range type"),
                }
        );
        OFFSETS[range_index as usize] * 0x8000
            + if range_type == NEG { value - 0x8000 } else { value } as usize
    }

    /// Calculates the row index for a carry range check based on the value.
    ///
    /// # Arguments
    /// * `value` - The carry value being validated.
    ///
    /// # Returns
    /// The calculated row index for the carry range table.
    pub fn get_row_carry_range_check(value: i64) -> usize {
        assert!(value >= -0xEFFFF);
        assert!(value <= 0xF0000);
        (0x220000 + 0xEFFFF + value) as usize
    }
}

/// The `ArithRangeTableInputs` struct manages row-specific multiplicity data for range checks.
/// It includes both direct storage and overflow handling for high-frequency rows.
pub struct ArithRangeTableInputs {
    // TODO: check improvement of multiplicity[64] to reserv only chunks used
    // with this 16 bits version, this table has aprox 8MB.
    updated: u64,
    multiplicity_overflow: HashMap<u32, u32>,
    multiplicity: Vec<u16>,
}

/// Provides a default implementation for `ArithRangeTableInputs`.
impl Default for ArithRangeTableInputs {
    fn default() -> Self {
        Self::new()
    }
}

impl ArithRangeTableInputs {
    /// Creates a new `ArithRangeTableInputs` instance.
    pub fn new() -> Self {
        ArithRangeTableInputs {
            updated: 0,
            multiplicity_overflow: HashMap::new(),
            multiplicity: vec![0u16; ROWS],
        }
    }

    /// Increments the multiplicity for a single row by one, handling overflow if needed.
    ///
    /// # Arguments
    /// * `row` - The row index to increment.
    fn incr_row_one(&mut self, row: usize) {
        if self.multiplicity[row] > u16::MAX - 1 {
            let count = self.multiplicity_overflow.entry(row as u32).or_insert(0);
            *count += 1;
            self.multiplicity[row] = 0;
        } else {
            self.multiplicity[row] += 1;
        }
        self.updated &= 1 << (row >> (22 - 6));
    }

    /// Increments the multiplicity for a row by a specified number of times.
    ///
    /// # Arguments
    /// * `row` - The row index to increment.
    /// * `times` - The number of times to increment.
    fn incr_row(&mut self, row: usize, times: usize) {
        self.incr_row_without_update(row, times);
        self.updated &= 1 << (row >> (22 - 6));
    }

    /// Increments the multiplicity for a row without updating the `updated` bitmask.
    ///
    /// # Arguments
    /// * `row` - The row index to increment.
    /// * `times` - The number of times to increment.
    fn incr_row_without_update(&mut self, row: usize, times: usize) {
        if (u16::MAX - self.multiplicity[row]) as usize <= times {
            let count = self.multiplicity_overflow.entry(row as u32).or_insert(0);
            let new_count = self.multiplicity[row] as u64 + times as u64;
            *count += (new_count >> 16) as u32;
            self.multiplicity[row] = (new_count & 0xFFFF) as u16;
        } else {
            self.multiplicity[row] += times as u16;
        }
    }

    /// Uses a chunk range check by incrementing the multiplicity for the calculated row.
    ///
    /// # Arguments
    /// * `range_id` - The range index to use.
    /// * `value` - The value to validate and use.
    pub fn use_chunk_range_check(&mut self, range_id: u8, value: u64) {
        let row = ArithRangeTableHelpers::get_row_chunk_range_check(range_id, value);
        self.incr_row_one(row);
    }

    /// Uses a carry range check by incrementing the multiplicity for the calculated row.
    ///
    /// # Arguments
    /// * `value` - The carry value to validate and use.
    pub fn use_carry_range_check(&mut self, value: i64) {
        let row = ArithRangeTableHelpers::get_row_carry_range_check(value);
        self.incr_row_one(row);
    }

    /// Uses a chunk range check multiple times by incrementing the multiplicity.
    ///
    /// # Arguments
    /// * `times` - The number of times to increment.
    /// * `range_id` - The range index to use.
    /// * `value` - The value to validate and use.
    pub fn multi_use_chunk_range_check(&mut self, times: usize, range_id: u8, value: u64) {
        let row = ArithRangeTableHelpers::get_row_chunk_range_check(range_id, value);
        self.incr_row(row, times);
    }

    /// Uses a carry range check multiple times by incrementing the multiplicity.
    ///
    /// # Arguments
    /// * `times` - The number of times to increment.
    /// * `value` - The carry value to validate and use.
    pub fn multi_use_carry_range_check(&mut self, times: usize, value: i64) {
        let row = ArithRangeTableHelpers::get_row_carry_range_check(value);
        self.incr_row(row, times);
    }

    /// Updates the current inputs with data from another `ArithRangeTableInputs`.
    ///
    /// # Arguments
    /// * `other` - The other `ArithRangeTableInputs` instance to merge.
    pub fn update_with(&mut self, other: &Self) {
        let chunk_size = 1 << (22 - 6);
        for i_chunk in 0..64 {
            if (other.updated & (1 << i_chunk)) == 0 {
                continue;
            }
            let from = chunk_size * i_chunk;
            let to = from + chunk_size;
            for row in from..to {
                let count = other.multiplicity[row];
                if count > 0 {
                    self.incr_row_without_update(row, count as usize);
                }
            }
        }
        for (row, value) in other.multiplicity_overflow.iter() {
            let count = self.multiplicity_overflow.entry(*row).or_insert(0);
            *count += (*value) << 16;
        }
        self.updated |= other.updated;
    }
}

/// Iterator for traversing rows in `ArithRangeTableInputs`.
pub struct ArithRangeTableInputsIterator<'a> {
    iter_row: u32,
    iter_hash: bool,
    inputs: &'a ArithRangeTableInputs,
}

impl Iterator for ArithRangeTableInputsIterator<'_> {
    type Item = (usize, u64);

    /// Retrieves the next row with non-zero multiplicity.
    fn next(&mut self) -> Option<Self::Item> {
        if !self.iter_hash {
            while self.iter_row < ROWS as u32
                && self.inputs.multiplicity[self.iter_row as usize] == 0
            {
                self.iter_row += 1;
            }
            let row = self.iter_row as usize;
            if row < ROWS {
                self.iter_row += 1;
                return Some((row, self.inputs.multiplicity[row] as u64));
            }
            self.iter_hash = true;
            self.iter_row = 0;
        }
        let res = self.inputs.multiplicity_overflow.iter().nth(self.iter_row as usize);
        match res {
            Some((row, value)) => {
                self.iter_row += 1;
                Some((*row as usize, (*value as u64) << 16))
            }
            None => None,
        }
    }
}

impl<'a> IntoIterator for &'a ArithRangeTableInputs {
    type Item = (usize, u64);
    type IntoIter = ArithRangeTableInputsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ArithRangeTableInputsIterator { iter_row: 0, iter_hash: false, inputs: self }
    }
}

#[cfg(feature = "generate_code_arith_range_table")]
#[allow(dead_code)]
fn generate_table() {
    let pattern = "FFF+++---FFFFFFFFF+-F+-F+-FFFFFFFFFFF+++---";
    // let mut ranges = [0u8; 43];
    let mut ranges = String::new();
    let mut offsets = [0usize; 43];
    let mut offset = 0;
    for range_loop in [FULL, POS, NEG] {
        let mut index = 0;
        for c in pattern.chars() {
            if c == ' ' || c == '_' {
                continue;
            }
            let range_id = match c {
                'F' => FULL,
                '+' => POS,
                '-' => NEG,
                _ => panic!("Invalid character in pattern"),
            };
            if range_loop == FULL {
                if index > 0 {
                    ranges.push_str(", ")
                }
                ranges.push_str(match range_id {
                    FULL => "FULL",
                    POS => "POS",
                    _ => "NEG",
                });
                // ranges[index] = range_id
            }
            if range_loop == range_id {
                offsets[index] = offset;
                offset += if range_loop == FULL { 2 } else { 1 };
            }
            index += 1;
        }
    }
    println!("const RANGES: [u8; 43] = [{}];", ranges);
    println!("const OFFSETS: [usize; 43] = {:?};", offsets);
}
