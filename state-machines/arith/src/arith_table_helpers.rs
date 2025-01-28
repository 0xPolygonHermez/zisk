//! This module defines helpers and inputs for managing arithmetic operations
//! and their associated tables, used in the context of Zero-Knowledge (ZK) computations.

/// The `ArithTableHelpers` struct provides utilities for retrieving row indices
/// from the arithmetic operation table based on operation codes and related flags.
///
/// It supports direct lookup for optimized retrieval in production and additional
/// debugging checks during testing.
pub struct ArithTableHelpers;

use crate::{ARITH_TABLE_ROWS, FIRST_OP, ROWS};

impl ArithTableHelpers {
    /// Retrieves the row index from the arithmetic table based on the provided operation and flags.
    ///
    /// # Arguments
    /// * `op` - The operation code.
    /// * `na` - Indicates whether the operand `a` is negative.
    /// * `nb` - Indicates whether the operand `b` is negative.
    /// * `np` - Indicates whether the result is negative.
    /// * `nr` - Indicates whether the remainder is negative.
    /// * `sext` - Indicates whether sign extension is enabled.
    /// * `div_by_zero` - Indicates whether a division-by-zero occurred.
    /// * `div_overflow` - Indicates whether a division overflow occurred.
    ///
    /// # Returns
    /// The row index corresponding to the operation and flags.
    #[allow(clippy::too_many_arguments)]
    pub fn direct_get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) -> usize {
        // Calculate the index into the ARITH_TABLE_ROWS lookup table.
        let index = (op - FIRST_OP) as u64 * 128
            + na as u64
            + nb as u64 * 2
            + np as u64 * 4
            + nr as u64 * 8
            + sext as u64 * 16
            + div_by_zero as u64 * 32
            + div_overflow as u64 * 64;

        // Ensure the index is within the valid range.
        debug_assert!(index < ARITH_TABLE_ROWS.len() as u64);

        // Retrieve the row index from the lookup table.
        let row = ARITH_TABLE_ROWS[index as usize];

        // Ensure the retrieved row is valid.
        debug_assert!(
            row < 255,
            "INVALID ROW row:{} op:0x{:x} na:{} nb:{} np:{} nr:{} sext:{} div_by_zero:{} div_overflow:{} index:{}",
            row,
            op,
            na as u8,
            nb as u8,
            np as u8,
            nr as u8,
            sext as u8,
            div_by_zero as u8,
            div_overflow as u8,
            index
        );
        row as usize
    }

    /// Retrieves the row index during testing (optimized for release mode).
    #[cfg(not(debug_assertions))]
    #[cfg(test)]
    pub fn get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) -> usize {
        Self::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow)
    }

    /// Retrieves the row index with additional debugging checks.
    ///
    /// This function validates the operation, flags, and ranges against a predefined
    /// arithmetic table during testing in debug mode.
    ///
    /// # Arguments
    /// - Same as `direct_get_row` with additional flags:
    /// * - `m32`: Indicates whether the operation uses 32-bit mode.
    /// * - `div`: Indicates whether the operation is a division.
    /// * - `main_mul`: Indicates whether the operation is the main multiplication.
    /// * - `main_div`: Indicates whether the operation is the main division.
    /// * - `signed`: Indicates whether the operation is signed.
    /// * - `range_ab`: The range of operands `a` and `b`.
    /// * - `range_cd`: The range of results `c` and `d`.
    ///
    /// # Returns
    /// The row index corresponding to the operation and flags.
    #[cfg(debug_assertions)]
    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub fn get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
        m32: bool,
        div: bool,
        main_mul: bool,
        main_div: bool,
        signed: bool,
        range_ab: u16,
        range_cd: u16,
    ) -> usize {
        use crate::ARITH_TABLE;

        // Calculate flags for the operation.
        let flags = if m32 { 1 } else { 0 }
            + if div { 2 } else { 0 }
            + if na { 4 } else { 0 }
            + if nb { 8 } else { 0 }
            + if np { 16 } else { 0 }
            + if nr { 32 } else { 0 }
            + if sext { 64 } else { 0 }
            + if div_by_zero { 128 } else { 0 }
            + if div_overflow { 256 } else { 0 }
            + if main_mul { 512 } else { 0 }
            + if main_div { 1024 } else { 0 }
            + if signed { 2048 } else { 0 };

        // Retrieve the row using the direct method.
        let row = Self::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow);

        // Validate the row against the ARITH_TABLE for correctness.
        assert_eq!(
            op as u16, ARITH_TABLE[row][0],
            "at row {} not match op {} vs {}",
            row, op, ARITH_TABLE[row][0]
        );
        assert_eq!(
            flags, ARITH_TABLE[row][1],
            "at row {0} op:0x{1:x}({1}) not match flags {2:b}({2}) vs {3:b}({3})",
            row, op, flags, ARITH_TABLE[row][1]
        );
        assert_eq!(
            range_ab, ARITH_TABLE[row][2],
            "at row {} op:{} not match range_ab {} vs {}",
            row, op, flags, ARITH_TABLE[row][2]
        );
        assert_eq!(
            range_cd, ARITH_TABLE[row][3],
            "at row {} op:{} not match range_cd {} vs {}",
            row, op, flags, ARITH_TABLE[row][3]
        );
        row
    }

    /// Converts operation flags into a human-readable string representation.
    ///
    /// # Arguments
    /// * - `flags`: A 16-bit integer representing operation flags.
    ///
    /// # Returns
    /// A string containing the human-readable representation of the flags.
    #[cfg(test)]
    pub fn flags_to_string(flags: u16) -> String {
        let mut result = String::new();
        if flags & 1 != 0 {
            result += " m32";
        }
        if flags & 2 != 0 {
            result += " div";
        }
        if flags & 4 != 0 {
            result += " na";
        }
        if flags & 8 != 0 {
            result += " nb";
        }
        if flags & 16 != 0 {
            result += " np";
        }
        if flags & 32 != 0 {
            result += " nr";
        }
        if flags & 64 != 0 {
            result += " sext";
        }
        if flags & 128 != 0 {
            result += " div_by_zero";
        }
        if flags & 256 != 0 {
            result += " div_overflow";
        }
        if flags & 512 != 0 {
            result += " main_mul";
        }
        if flags & 1024 != 0 {
            result += " main_div";
        }
        if flags & 2048 != 0 {
            result += " signed";
        }
        result
    }
}

/// The `ArithTableInputs` struct manages multiplicity values for rows in the
/// arithmetic operation table, enabling tracking and updates of operation usage.
pub struct ArithTableInputs {
    /// Multiplicity table
    multiplicity: [u64; ROWS],
}

/// Provides Default implementation for `ArithTableInputs`.
impl Default for ArithTableInputs {
    fn default() -> Self {
        Self::new()
    }
}

impl ArithTableInputs {
    /// Creates a new instance of `ArithTableInputs` with all multiplicity values initialized to
    /// zero.
    pub fn new() -> Self {
        ArithTableInputs { multiplicity: [0; ROWS] }
    }

    /// Updates the multiplicity for a specific operation and flags by incrementing it by 1.
    ///
    /// # Arguments
    /// * - `op`: The operation code.
    /// * - `na`, `nb`, `np`, `nr`, `sext`, `div_by_zero`, `div_overflow`: Operation flags.
    ///
    /// # Panics
    /// Panics if the row index exceeds the bounds of the multiplicity table.
    #[allow(clippy::too_many_arguments)]
    pub fn add_use(
        &mut self,
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) {
        let row =
            ArithTableHelpers::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow);
        assert!(row < ROWS);
        self.multiplicity[row] += 1;
    }

    /// Updates the multiplicity for a specific operation and flags by incrementing it by a given
    /// amount.
    ///
    /// # Arguments
    /// * - `times`: The number of times to increment the multiplicity.
    /// * - `op`, `na`, `nb`, `np`, `nr`, `sext`, `div_by_zero`, `div_overflow`: Operation flags.
    ///
    /// # Panics
    /// Panics if the row index exceeds the bounds of the multiplicity table.
    #[allow(clippy::too_many_arguments)]
    pub fn multi_add_use(
        &mut self,
        times: usize,
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
    ) {
        let row =
            ArithTableHelpers::direct_get_row(op, na, nb, np, nr, sext, div_by_zero, div_overflow);
        self.multiplicity[row] += times as u64;
    }

    /// Merges multiplicity data from another `ArithTableInputs` instance.
    ///
    /// # Arguments
    /// * - `other`: The other `ArithTableInputs` instance to merge with.
    pub fn update_with(&mut self, other: &Self) {
        for i in 0..ROWS {
            self.multiplicity[i] += other.multiplicity[i];
        }
    }
}

/// The `ArithTableInputsIterator` struct implements an iterator for traversing
/// non-zero multiplicity values in the `ArithTableInputs` structure.
pub struct ArithTableInputsIterator<'a> {
    iter_row: u32,
    inputs: &'a ArithTableInputs,
}

impl Iterator for ArithTableInputsIterator<'_> {
    type Item = (usize, u64);

    /// Advances the iterator and retrieves the next non-zero multiplicity value with its row index.
    ///
    /// # Returns
    /// An `Option` containing a tuple `(row, multiplicity)` where:
    /// - `row`: The index of the row with a non-zero multiplicity.
    /// - `multiplicity`: The multiplicity value at the specified row.
    ///
    /// Returns `None` if all rows have been processed.
    ///
    /// # Behavior
    /// The iterator skips over rows with a multiplicity value of zero,
    /// continuing until it finds the next non-zero value or reaches the end of the table.
    fn next(&mut self) -> Option<Self::Item> {
        while self.iter_row < ROWS as u32 && self.inputs.multiplicity[self.iter_row as usize] == 0 {
            self.iter_row += 1;
        }
        let row = self.iter_row as usize;
        if row < ROWS {
            self.iter_row += 1;
            Some((row, self.inputs.multiplicity[row]))
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for &'a ArithTableInputs {
    type Item = (usize, u64);
    type IntoIter = ArithTableInputsIterator<'a>;

    /// Converts `ArithTableInputs` into an iterator for traversing non-zero multiplicity values.
    ///
    /// # Returns
    /// An iterator that yields `(row, multiplicity)` pairs.
    fn into_iter(self) -> Self::IntoIter {
        ArithTableInputsIterator { iter_row: 0, inputs: self }
    }
}
