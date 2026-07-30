//! This module defines helpers and inputs for managing arithmetic operations
//! and their associated tables, used in the context of Zero-Knowledge (ZK) computations.

/// The `ArithTableHelpers` struct provides utilities for retrieving row indices
/// from the arithmetic operation table based on operation codes and related flags.
///
/// It supports direct lookup for optimized retrieval in production and additional
/// debugging checks during testing.
pub struct ArithTableHelpers;

use crate::{ARITH_TABLE_ROWS, FIRST_OP, ICASES, ROWS};

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
        result_is_zero: bool,
        remainder_is_zero: bool,
    ) -> usize {
        // Calculate the index into the ARITH_TABLE_ROWS lookup table. The 9 flag bits must be laid
        // out exactly as `icase` in pil/arith_table.pil.
        let index = (op - FIRST_OP) as u64 * ICASES as u64
            + na as u64
            + nb as u64 * 2
            + np as u64 * 4
            + nr as u64 * 8
            + sext as u64 * 16
            + div_by_zero as u64 * 32
            + div_overflow as u64 * 64
            + result_is_zero as u64 * 128
            + remainder_is_zero as u64 * 256;

        // Ensure the index is within the valid range.
        debug_assert!(index < ARITH_TABLE_ROWS.len() as u64);

        // Retrieve the row index from the lookup table.
        let row = ARITH_TABLE_ROWS[index as usize];

        // Ensure the retrieved row is valid.
        debug_assert!(
            row < 255,
            "INVALID ROW row:{} op:0x{:x} na:{} nb:{} np:{} nr:{} sext:{} div_by_zero:{} div_overflow:{} result_is_zero:{} remainder_is_zero:{} index:{}",
            row,
            op,
            na as u8,
            nb as u8,
            np as u8,
            nr as u8,
            sext as u8,
            div_by_zero as u8,
            div_overflow as u8,
            result_is_zero as u8,
            remainder_is_zero as u8,
            index
        );
        row as usize
    }

    /// Retrieves the row index during testing (optimized for release mode).
    #[cfg(not(debug_assertions))]
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn get_row(
        op: u8,
        na: bool,
        nb: bool,
        np: bool,
        nr: bool,
        sext: bool,
        div_by_zero: bool,
        div_overflow: bool,
        result_is_zero: bool,
        remainder_is_zero: bool,
    ) -> usize {
        Self::direct_get_row(
            op,
            na,
            nb,
            np,
            nr,
            sext,
            div_by_zero,
            div_overflow,
            result_is_zero,
            remainder_is_zero,
        )
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
        result_is_zero: bool,
        remainder_is_zero: bool,
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
            + if signed { 2048 } else { 0 }
            + if result_is_zero { 4096 } else { 0 }
            + if remainder_is_zero { 8192 } else { 0 };

        // Retrieve the row using the direct method.
        let row = Self::direct_get_row(
            op,
            na,
            nb,
            np,
            nr,
            sext,
            div_by_zero,
            div_overflow,
            result_is_zero,
            remainder_is_zero,
        );

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
        if flags & 4096 != 0 {
            result += " result_is_zero";
        }
        if flags & 8192 != 0 {
            result += " remainder_is_zero";
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
        result_is_zero: bool,
        remainder_is_zero: bool,
    ) {
        let row = ArithTableHelpers::direct_get_row(
            op,
            na,
            nb,
            np,
            nr,
            sext,
            div_by_zero,
            div_overflow,
            result_is_zero,
            remainder_is_zero,
        );
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
        result_is_zero: bool,
        remainder_is_zero: bool,
    ) {
        let row = ArithTableHelpers::direct_get_row(
            op,
            na,
            nb,
            np,
            nr,
            sext,
            div_by_zero,
            div_overflow,
            result_is_zero,
            remainder_is_zero,
        );
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

#[cfg(test)]
mod table_coverage_tests {
    //! Locks in the two invariants of ArithTable:
    //!
    //! * completeness - every real operation has a row. `direct_get_row` returns 255 and
    //!   `add_use` asserts if it does not, so simply exercising the executor covers this.
    //! * soundness - every row corresponds to some real operation. A row no operation can produce
    //!   is a row a prover could try to misuse, so the table must not contain any.
    //!
    //! This works from the generated table and the executor only: it does not re-transcribe the
    //! filters of `pil/arith_table.pil`, so it stays valid when those change (regenerate the table
    //! and re-run).

    use super::*;
    use crate::{ArithOperation, ARITH_TABLE};
    use zisk_core::zisk_ops::ZiskOp;

    const ALL_OPS: [ZiskOp; 14] = [
        ZiskOp::Mulu,
        ZiskOp::Muluh,
        ZiskOp::Mulsuh,
        ZiskOp::Mul,
        ZiskOp::Mulh,
        ZiskOp::MulW,
        ZiskOp::Divu,
        ZiskOp::Remu,
        ZiskOp::Div,
        ZiskOp::Rem,
        ZiskOp::DivuW,
        ZiskOp::RemuW,
        ZiskOp::DivW,
        ZiskOp::RemW,
    ];

    fn is_w(op: ZiskOp) -> bool {
        matches!(op, ZiskOp::MulW | ZiskOp::DivuW | ZiskOp::RemuW | ZiskOp::DivW | ZiskOp::RemW)
    }

    /// Values chosen to hit every corner the flags can distinguish: zero, one, both signs, the
    /// 32- and 64-bit extremes, values with bit 31 or bit 63 set, and divisors above 2^31 (which is
    /// what makes a quotient zero with a large remainder).
    fn sweep_values() -> Vec<u64> {
        let mut v: Vec<u64> = (0i64..=40).map(|x| x as u64).collect();
        for x in -40i64..0 {
            v.push(x as u64);
        }
        for x in [
            i64::MIN,
            i64::MIN + 1,
            i64::MAX,
            i64::MAX - 1,
            i32::MIN as i64,
            i32::MIN as i64 + 1,
            i32::MAX as i64,
            i32::MAX as i64 - 1,
            1 << 15,
            1 << 16,
            1 << 31,
            1 << 32,
            1 << 47,
            1 << 62,
            -(1 << 15),
            -(1 << 16),
            -(1 << 31),
            -(1 << 32),
        ] {
            v.push(x as u64);
        }
        v.extend([u64::MAX, 0xFFFF_FFFF, 0xFFFF_FFFE, 0x8000_0000, 0x8000_0001, 0xC000_0000]);
        v.sort_unstable();
        v.dedup();
        v
    }

    #[test]
    fn every_row_is_reachable_and_every_operation_has_a_row() {
        let values = sweep_values();
        let mut hits = [0usize; ROWS];
        let mut aop = ArithOperation::new();
        let mut runs = 0usize;

        for &op in ALL_OPS.iter() {
            for &a in &values {
                for &b in &values {
                    // the _W ops take their operands already reduced to 32 bits
                    let (a, b) = if is_w(op) {
                        (((a as i32) as u32) as u64, ((b as i32) as u32) as u64)
                    } else {
                        (a, b)
                    };
                    aop.calculate(op.code(), a, b);
                    runs += 1;

                    // completeness: panics (row == 255) if this operation has no row
                    let row = ArithTableHelpers::direct_get_row(
                        op.code(),
                        aop.na,
                        aop.nb,
                        aop.np,
                        aop.nr,
                        aop.sext,
                        aop.div_by_zero,
                        aop.div_overflow,
                        aop.result_is_zero,
                        aop.remainder_is_zero,
                    );
                    assert!(row < ROWS, "{op:?} {a:#x}/{b:#x} maps outside the table");
                    assert_eq!(
                        op.code() as u16,
                        ARITH_TABLE[row][0],
                        "{op:?} {a:#x}/{b:#x} landed on a row of another opcode"
                    );
                    hits[row] += 1;
                }
            }
        }

        // soundness: no row may be left over
        let unreachable: Vec<usize> = (0..ROWS).filter(|&r| hits[r] == 0).collect();
        if !unreachable.is_empty() {
            for r in &unreachable {
                println!(
                    "  unreachable row {r}: op=0x{:x} flags={}",
                    ARITH_TABLE[*r][0],
                    ArithTableHelpers::flags_to_string(ARITH_TABLE[*r][1])
                );
            }
        }
        println!("{runs} operations covered all {ROWS} rows");
        assert!(
            unreachable.is_empty(),
            "{} table rows are unreachable: {unreachable:?}",
            unreachable.len()
        );
    }
}

#[cfg(test)]
mod padding_row_tests {
    //! The ArithTable and ArithRangeTable lookups in `arith.pil` have no selector, so every row of
    //! the Arith trace must match a table entry - including the padding rows that `ArithFullSM`
    //! writes after the real operations.
    //!
    //! Those rows start from `R::default()` (all zeros) and then write only `op`, `main_mul`,
    //! `result_is_zero`, `range_ab` and `range_cd`. That is a contract with two halves, and both are
    //! checked here: which columns must be written, and that nothing else needs to be.
    //!
    //! It used to hold by accident - the all-FULL range id was 0, so an all-zero row happened to be
    //! a valid entry. After the range-id renumbering it is 3 and the all-zero row stopped matching,
    //! which showed up as 2M unmatched lookups on opid 330/331.
    //!
    //! NOTE: these tests pin the contract, they do not run `compute_witness`. The end-to-end check
    //! is a constraint-verification run.

    use super::*;
    use crate::{ArithOperation, ArithRangeTableHelpers, ARITH_RANGE_16_BITS, ARITH_TABLE};
    use zisk_core::zisk_ops::ZiskOp;

    /// The padding operation, which must stay in sync with `ArithFullSM::compute_witness`.
    fn padding_operation() -> ArithOperation {
        let mut pad = ArithOperation::new();
        pad.calculate(ZiskOp::Mulu.code(), 0, 0);
        pad
    }

    #[test]
    fn the_padding_operation_has_a_table_row() {
        let pad = padding_operation();
        let row = ArithTableHelpers::direct_get_row(
            pad.op,
            pad.na,
            pad.nb,
            pad.np,
            pad.nr,
            pad.sext,
            pad.div_by_zero,
            pad.div_overflow,
            pad.result_is_zero,
            pad.remainder_is_zero,
        );
        assert!(row < ROWS, "the padding operation has no row in ArithTable");
        assert_eq!(pad.op as u16, ARITH_TABLE[row][0]);
        assert_eq!(pad.range_ab as u16, ARITH_TABLE[row][2]);
        assert_eq!(pad.range_cd as u16, ARITH_TABLE[row][3]);
    }

    /// The half that actually catches a forgotten `set_*`: every column of the padding row that is
    /// NOT explicitly written by `ArithFullSM` must be zero for this operation. If a change makes one
    /// of them non-zero, the padding row silently stops matching its table entry, so fail here and
    /// name the column that needs writing.
    #[test]
    fn nothing_beyond_the_written_columns_must_be_set() {
        let pad = padding_operation();
        let zero_flags: [(&str, bool); 11] = [
            ("m32", pad.m32),
            ("div", pad.div),
            ("na", pad.na),
            ("nb", pad.nb),
            ("np", pad.np),
            ("nr", pad.nr),
            ("sext", pad.sext),
            ("div_by_zero", pad.div_by_zero),
            ("div_overflow", pad.div_overflow),
            ("remainder_is_zero", pad.remainder_is_zero),
            ("main_div", pad.main_div),
        ];
        for (name, value) in zero_flags {
            assert!(
                !value,
                "the padding operation sets `{name}`, so the padding row in ArithFullSM must \
                 write it explicitly instead of relying on R::default()"
            );
        }
        assert!(!pad.signed, "the padding operation is signed; ArithFullSM must write `signed`");
        for (name, chunks) in [("a", pad.a), ("b", pad.b), ("c", pad.c), ("d", pad.d)] {
            assert_eq!(chunks, [0u16; 4], "the padding operation has non-zero `{name}` chunks");
        }
        assert_eq!(pad.carry, [0i64; 7], "the padding operation has non-zero carries");
    }

    #[test]
    fn every_range_id_the_padding_row_uses_accepts_zero() {
        // All of the padding row's chunks are zero, so every range slot it looks up has to admit the
        // value 0 - none of them may be a NEG slot. `get_row_chunk_range_check` panics if it is.
        let pad = padding_operation();
        ArithRangeTableHelpers::get_row_chunk_range_check(ARITH_RANGE_16_BITS, 0);
        for offset in 0..4 {
            ArithRangeTableHelpers::get_row_chunk_range_check(pad.range_ab + offset, 0);
            ArithRangeTableHelpers::get_row_chunk_range_check(pad.range_cd + offset, 0);
        }
        ArithRangeTableHelpers::get_row_carry_range_check(0);
    }
}
