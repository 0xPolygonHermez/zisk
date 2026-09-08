//! Publishing the FROPS multiplicity column that the ROM-histogram assembly builds.
//!
//! A frequent operation is proved as one row of a fixed lookup table whose multiplicity counts how
//! many times that `(op, a, b)` triple was executed. There are exactly two producers of that column:
//!
//! * the state machines, which accumulate it row by row as they collect each instance
//!   (`inc_virtual_row_one` in the binary / arith collectors), on **every** worker;
//! * the ROM-histogram assembly, which counts every executed operation in one sequential pass on a
//!   single worker (`zisk_core::frops_asm`), and hands the whole column over in
//!   [`zisk_asm_runner::AsmRHData::frops_count`].
//!
//! This module publishes the second one. The two are mutually exclusive: publishing the assembly
//! column while the collectors also accumulate would double every multiplicity and the lookup
//! argument would not balance, which is why [`super::StaticSMBundle`] keeps it behind a flag that is
//! off until the collectors are told to stand down.
//!
//! The column is global — the three family tables concatenated in `FROPS_*_BASE` order — while each
//! family AIR indexes its own rows, so it is split at the family bases and published relative to
//! each.

use pil2_std_lib::Std;
use proofman_fields::PrimeField64;

use zisk_core::frops::{
    FROPS_ARITH_BASE, FROPS_BINARY_BASIC_BASE, FROPS_BINARY_EXT_BASE, FROPS_TABLE_ROWS,
};
use zisk_sm_arith::ArithFrops;
use zisk_sm_binary::{BinaryBasicFrops, BinaryExtensionFrops};

use crate::error::{ExecutorError, ExecutorResult};

/// Rows published per call. `inc_virtual_rows_ranged` copies its slice into a temporary buffer, so
/// the column is fed in chunks to keep that buffer small instead of allocating another copy of the
/// whole table.
const CHUNK_ROWS: usize = 1 << 20;

/// Environment variable that arms the debug cross-check of the two producers of the column
/// (`zisk_core::frops`). Off by default: it makes the collectors compute the table row even when
/// they no longer publish it.
pub const CROSS_CHECK_ENV: &str = "ZISK_FROPS_CROSS_CHECK";

/// The three family tables of the global column: table id, and the half-open range of global rows it
/// owns.
const FAMILIES: [(usize, u64, u64); 3] = [
    (ArithFrops::TABLE_ID, FROPS_ARITH_BASE, FROPS_BINARY_BASIC_BASE),
    (BinaryBasicFrops::TABLE_ID, FROPS_BINARY_BASIC_BASE, FROPS_BINARY_EXT_BASE),
    (BinaryExtensionFrops::TABLE_ID, FROPS_BINARY_EXT_BASE, FROPS_TABLE_ROWS),
];

/// Splits the global column into one slice per family table, checking it has the expected length.
///
/// A length mismatch means the assembly was generated from different FROPS data than the state
/// machines were built with, which would silently produce a wrong multiplicity column.
fn family_slices(column: &[u64]) -> ExecutorResult<[(usize, &[u64]); 3]> {
    if column.len() as u64 != FROPS_TABLE_ROWS {
        return Err(ExecutorError::Internal(format!(
            "the assembly delivered {} FROPS counters but the in-tree table has {FROPS_TABLE_ROWS} \
             rows; the assembly was generated from different FROPS data",
            column.len()
        )));
    }
    Ok(FAMILIES.map(|(table_id, base, end)| (table_id, &column[base as usize..end as usize])))
}

/// Publishes the whole column into the three family virtual tables.
pub fn publish_frops_multiplicity<F: PrimeField64>(
    std: &Std<F>,
    column: &[u64],
) -> ExecutorResult<()> {
    for (table_id, family) in family_slices(column)? {
        let id = std.get_virtual_table_id(table_id).map_err(|e| {
            ExecutorError::Internal(format!("no virtual table for FROPS table id {table_id}: {e}"))
        })?;
        for (chunk_idx, chunk) in family.chunks(CHUNK_ROWS).enumerate() {
            let start = (chunk_idx * CHUNK_ROWS) as u64;
            std.inc_virtual_rows_ranged(id, Some(start), chunk);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The split must cover the whole column exactly once, in family order, with no gap or overlap.
    #[test]
    fn family_slices_partition_the_column() {
        let column = vec![0u64; FROPS_TABLE_ROWS as usize];
        let slices = family_slices(&column).expect("the column has the expected length");
        let total: usize = slices.iter().map(|(_, s)| s.len()).sum();
        assert_eq!(total, column.len(), "the families do not cover the column");

        let mut expected_start = 0usize;
        for (_, family) in &slices {
            let start = family.as_ptr() as usize - column.as_ptr() as usize;
            assert_eq!(start / 8, expected_start, "families are not contiguous");
            expected_start += family.len();
        }

        let ids: Vec<usize> = slices.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2], "table ids repeat");
    }

    /// A column of the wrong length must be rejected, not silently published.
    #[test]
    fn a_column_of_the_wrong_length_is_rejected() {
        for len in [0, 1, FROPS_TABLE_ROWS as usize - 1, FROPS_TABLE_ROWS as usize + 1] {
            let err = family_slices(&vec![0u64; len]).expect_err("should be rejected");
            assert!(
                err.to_string().contains("different FROPS data"),
                "unexpected error for length {len}: {err}"
            );
        }
    }
}
