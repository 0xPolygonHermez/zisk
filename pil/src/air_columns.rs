//! Per-air setup column counts — the width the prover actually pays for.
//!
//! Planners price an instance by its **area**: `NUM_ROWS × setup_columns(air_id)`. `NUM_ROWS` comes
//! from the generated trace types, so the only figure that cannot be derived from `zisk_pil` alone is
//! the column count, which is what this table declares.
//!
//! It is the same figure the executor prices instances with at run time — the sum of
//! `stark_info.map_sections_n` over every section but `const` (see `setup_cost` in
//! `executor/src/adapters.rs`) — and it exceeds the committed width of a trace row because it counts
//! the stage-2 and quotient columns too.
//!
//! # Where the numbers come from
//!
//! The PIL compiler reports, for every air, `witness: <stage1>,<stage2>` (see `build/compile.log`).
//! Stage-2 columns live in the cubic extension, so each costs three field elements, and the quotient
//! section adds [`Q_COLUMNS`] more:
//!
//! ```text
//! setup_columns = stage1 + 3 × stage2 + Q_COLUMNS
//! ```
//!
//! **Refresh this table whenever an air's columns change**, by re-reading the `witness:` figures from
//! a fresh PIL compilation. [`columns_cover_the_committed_row`] catches the most likely form of
//! staleness — a count that no longer even covers the trace's committed width — but it cannot see the
//! stage-2 columns, so an air that grows only those will not be flagged.

/// Field elements the quotient section contributes to every air.
///
/// Verified against the setup-derived counts that used to be declared by hand in the planners:
/// `Binary` 39+3×5+6 = 60, `BinaryAdd` 10+3×3+6 = 25, `BinaryAddHi` 15+3×5+6 = 36,
/// `BinaryExtension` 34+3×6+6 = 58, `MemAlignReadByte` 10+3×3+6 = 25 and
/// `MemAlignWriteByte` 14+3×4+6 = 32 all reproduce exactly.
const Q_COLUMNS: u64 = 6;

/// `(stage1, stage2)` witness columns of every air, indexed by air id.
///
/// The order is the air order of the pilout, i.e. the order of the `AIR_IDS` consts in
/// [`crate::pil_helpers`]; [`air_ids_match_the_generated_consts`] pins it.
const WITNESS_COLUMNS: [(u64, u64); 48] = [
    (38, 8),   // 0  Main
    (1, 3),    // 1  Rom
    (13, 3),   // 2  Mem
    (9, 4),    // 3  InputData
    (5, 1),    // 4  RomData
    (32, 4),   // 5  MemAlign
    (32, 4),   // 6  MemAlignLarge
    (16, 4),   // 7  MemAlignByte
    (10, 3),   // 8  MemAlignReadByte
    (14, 4),   // 9  MemAlignWriteByte
    (16, 4),   // 10 MemAlignByteLarge
    (10, 3),   // 11 MemAlignReadByteLarge
    (45, 15),  // 12 Arith
    (39, 5),   // 13 Binary
    (39, 5),   // 14 BinaryLarge
    (10, 3),   // 15 BinaryAdd
    (10, 3),   // 16 BinaryAddLarge
    (15, 5),   // 17 BinaryAddHi
    (25, 8),   // 18 BinaryAddHiLarge
    (34, 6),   // 19 BinaryExtension
    (34, 6),   // 20 BinaryExtensionLarge
    (47, 17),  // 21 Add256
    (45, 12),  // 22 ArithEq
    (45, 12),  // 23 ArithEqLarge
    (19, 9),   // 24 Arith256X
    (19, 9),   // 25 Arith256XLarge
    (27, 12),  // 26 ArithSecp256K1
    (27, 12),  // 27 ArithSecp256K1Large
    (33, 12),  // 28 ArithBn254
    (33, 12),  // 29 ArithBn254Large
    (35, 12),  // 30 ArithEq384
    (35, 12),  // 31 ArithEq384Large
    (39, 20),  // 32 BabyJubJub
    (453, 61), // 33 Keccakf
    (102, 3),  // 34 Sha256f
    (84, 37),  // 35 Poseidon
    (119, 34), // 36 Blake2br
    (114, 32), // 37 Blake3f
    (34, 7),   // 38 Dma
    (35, 12),  // 39 Dma64Aligned
    (35, 12),  // 40 Dma64AlignedLarge
    (14, 5),   // 41 Dma64AlignedMemSet
    (26, 6),   // 42 Dma64AlignedMem
    (26, 6),   // 43 Dma64AlignedMemLarge
    (31, 10),  // 44 Dma64AlignedMemCpy
    (24, 4),   // 45 DmaUnaligned
    (66, 10),  // 46 DmaPrePost
    (32, 8),   // 47 JumpDest
];

/// Columns the setup commits for `air_id`, stage-2 counted in field elements.
///
/// # Panics
/// Panics if `air_id` is not one of the airs this table covers — the virtual tables (47, 48) are
/// planned apart and have no entry.
#[inline]
pub const fn setup_columns(air_id: usize) -> u64 {
    let (stage1, stage2) = WITNESS_COLUMNS[air_id];
    stage1 + 3 * stage2 + Q_COLUMNS
}

/// Area of one instance of `air_id` holding `num_rows` rows: what the planners minimise once the
/// instance count is settled.
#[inline]
pub const fn instance_area(air_id: usize, num_rows: usize) -> u64 {
    setup_columns(air_id) * num_rows as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pil_helpers::*;
    use proofman_common::trace::TraceRow;
    use proofman_fields::Goldilocks;

    /// The table is indexed by air id, so its order must be the pilout's. Spot-checking the first,
    /// last and a few airs in between is enough to catch a table shifted by an inserted alias.
    #[test]
    fn air_ids_match_the_generated_consts() {
        assert_eq!(WITNESS_COLUMNS.len(), JUMP_DEST_AIR_IDS[0] + 1);
        assert_eq!(MAIN_AIR_IDS[0], 0);
        assert_eq!(BINARY_AIR_IDS[0], 13);
        assert_eq!(ARITH_EQ_AIR_IDS[0], 22);
        assert_eq!(DMA_AIR_IDS[0], 38);
    }

    /// Each declared count must cover at least the committed width of its trace row — the setup counts
    /// those columns plus the stage-2 and quotient ones. A count that no longer does is stale.
    #[test]
    fn columns_cover_the_committed_row() {
        macro_rules! check {
            ($( $trace:ident : $row:ident ),+ $(,)?) => {$(
                let air_id = $trace::<()>::AIR_ID;
                let row_size = $row::<Goldilocks>::ROW_SIZE as u64;
                assert!(
                    setup_columns(air_id) >= row_size,
                    "the setup width of {} ({}) no longer covers its {row_size} committed columns: \
                     the air gained columns and the table was not refreshed",
                    stringify!($trace),
                    setup_columns(air_id),
                );
            )+};
        }
        check!(
            MemTrace: MemTraceRow,
            MemAlignTrace: MemAlignTraceRow,
            MemAlignLargeTrace: MemAlignLargeTraceRow,
            MemAlignByteTrace: MemAlignByteTraceRow,
            MemAlignReadByteTrace: MemAlignReadByteTraceRow,
            MemAlignWriteByteTrace: MemAlignWriteByteTraceRow,
            MemAlignByteLargeTrace: MemAlignByteLargeTraceRow,
            MemAlignReadByteLargeTrace: MemAlignReadByteLargeTraceRow,
            ArithTrace: ArithTraceRow,
            Blake2brTrace: Blake2brTraceRow,
            Blake3fTrace: Blake3fTraceRow,
            BinaryTrace: BinaryTraceRow,
            BinaryLargeTrace: BinaryLargeTraceRow,
            BinaryAddTrace: BinaryAddTraceRow,
            BinaryAddLargeTrace: BinaryAddLargeTraceRow,
            BinaryAddHiTrace: BinaryAddHiTraceRow,
            BinaryAddHiLargeTrace: BinaryAddHiLargeTraceRow,
            BinaryExtensionTrace: BinaryExtensionTraceRow,
            BinaryExtensionLargeTrace: BinaryExtensionLargeTraceRow,
            ArithEqTrace: ArithEqTraceRow,
            ArithEqLargeTrace: ArithEqLargeTraceRow,
            Arith256XTrace: Arith256XTraceRow,
            Arith256XLargeTrace: Arith256XLargeTraceRow,
            ArithSecp256K1Trace: ArithSecp256K1TraceRow,
            ArithSecp256K1LargeTrace: ArithSecp256K1LargeTraceRow,
            ArithBn254Trace: ArithBn254TraceRow,
            ArithBn254LargeTrace: ArithBn254LargeTraceRow,
            ArithEq384Trace: ArithEq384TraceRow,
            ArithEq384LargeTrace: ArithEq384LargeTraceRow,
            DmaTrace: DmaTraceRow,
            Dma64AlignedTrace: Dma64AlignedTraceRow,
            Dma64AlignedLargeTrace: Dma64AlignedLargeTraceRow,
            Dma64AlignedMemSetTrace: Dma64AlignedMemSetTraceRow,
            Dma64AlignedMemTrace: Dma64AlignedMemTraceRow,
            Dma64AlignedMemLargeTrace: Dma64AlignedMemLargeTraceRow,
            Dma64AlignedMemCpyTrace: Dma64AlignedMemCpyTraceRow,
            DmaUnalignedTrace: DmaUnalignedTraceRow,
            DmaPrePostTrace: DmaPrePostTraceRow,
            JumpDestTrace: JumpDestTraceRow,
        );
    }
}
