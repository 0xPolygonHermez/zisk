//! What one instance of each air costs, so the planners can price a layout.
//!
//! One constant per air, in **MB of prover memory**. The value is the whole instance regardless of
//! how full it ends up: an instance costs the same either way.
//!
//! **This file is meant to be regenerated or edited by hand.** Nothing derives these numbers at
//! build time, precisely so that they can be overridden: raising an air's cost steers the planners
//! away from it, and lowering it steers them towards it, without touching any strategy.
//!
//! # A cost is bound to a name, never to an air id
//!
//! There is deliberately no table indexed by air id here, and no `cost_of(air_id)`. Air ids are
//! positional — inserting one air in `zisk.pil` shifts every id after it — so a positional table
//! would silently reassign every cost below the new air. Callers name the constant of the air they
//! mean; the ones that only have an id are handed the cost by whoever knew the air statically.
//!
//! # Where the numbers come from
//!
//! The peak memory the prover needs to prove one instance, which is what actually bounds how many
//! can be proved at once and therefore the closest thing to what an instance really costs. It is
//! not the committed trace: the extended-domain evaluations, the Merkle trees and the FRI folding
//! steps all dwarf it, and they do not grow with the trace in the same proportion — an air with few
//! columns but a high constraint degree can cost more than a wider one.
//!
//! The setup prints it per air, so these are measured rather than modelled. Each `SUMMARY` line of
//! `build/setup.log` ends with it:
//!
//! ```text
//! SUMMARY | Binary | nBits: 22 | ... | Prover memory: 6.51 GB
//! ```
//!
//! To regenerate after a PIL change, run the setup and convert each air's figure to MB. The log
//! reports GiB with two decimals, so `MB = round(GB * 1024)` and the resolution is about 10 MB —
//! far finer than any decision the planners make with it.
//!
//! Note the unit is shared by every planner that compares costs, so what matters is that all of
//! them move together: mixing a memory cost with a column-area one would make the comparison
//! meaningless.

/// `Main`: 14.22 GB.
pub const MAIN_INSTANCE_COST: usize = 14561;

/// `Rom`: 5.39 GB.
pub const ROM_INSTANCE_COST: usize = 5519;

/// `Mem`: 11.78 GB.
pub const MEM_INSTANCE_COST: usize = 12063;

/// `InputData`: 4.58 GB.
pub const INPUT_DATA_INSTANCE_COST: usize = 4690;

/// `RomData`: 4.14 GB.
pub const ROM_DATA_INSTANCE_COST: usize = 4239;

/// `MemAlign`: 2.94 GB.
pub const MEM_ALIGN_INSTANCE_COST: usize = 3011;

/// `MemAlignLarge`: 11.78 GB.
pub const MEM_ALIGN_LARGE_INSTANCE_COST: usize = 12063;

/// `MemAlignByte`: 4.89 GB.
pub const MEM_ALIGN_BYTE_INSTANCE_COST: usize = 5007;

/// `MemAlignReadByte`: 4.33 GB.
pub const MEM_ALIGN_READ_BYTE_INSTANCE_COST: usize = 4434;

/// `MemAlignWriteByte`: 4.76 GB.
pub const MEM_ALIGN_WRITE_BYTE_INSTANCE_COST: usize = 4874;

/// `MemAlignByteLarge`: 9.78 GB.
pub const MEM_ALIGN_BYTE_LARGE_INSTANCE_COST: usize = 10015;

/// `MemAlignReadByteLarge`: 8.65 GB.
pub const MEM_ALIGN_READ_BYTE_LARGE_INSTANCE_COST: usize = 8858;

/// `Arith`: 4.41 GB.
pub const ARITH_INSTANCE_COST: usize = 4516;

/// `Binary`: 6.51 GB.
pub const BINARY_INSTANCE_COST: usize = 6666;

/// `BinaryLarge`: 10.01 GB.
pub const BINARY_LARGE_INSTANCE_COST: usize = 10250;

/// `BinaryHuge`: 18.17 GB.
pub const BINARY_HUGE_INSTANCE_COST: usize = 18606;

/// `BinaryAdd`: 4.39 GB.
pub const BINARY_ADD_INSTANCE_COST: usize = 4495;

/// `BinaryAddLarge`: 5.64 GB.
pub const BINARY_ADD_LARGE_INSTANCE_COST: usize = 5775;

/// `BinaryAddHuge`: 7.95 GB.
pub const BINARY_ADD_HUGE_INSTANCE_COST: usize = 8141;

/// `BinaryAddHi`: 4.64 GB.
pub const BINARY_ADD_HI_INSTANCE_COST: usize = 4751;

/// `BinaryAddHiLarge`: 5.95 GB.
pub const BINARY_ADD_HI_LARGE_INSTANCE_COST: usize = 6093;

/// `BinaryAddHiHuge`: 8.58 GB.
pub const BINARY_ADD_HI_HUGE_INSTANCE_COST: usize = 8786;

/// `BinaryExtension`: 6.39 GB.
pub const BINARY_EXTENSION_INSTANCE_COST: usize = 6543;

/// `BinaryExtensionLarge`: 9.64 GB.
pub const BINARY_EXTENSION_LARGE_INSTANCE_COST: usize = 9871;

/// `BinaryExtensionHuge`: 16.67 GB.
pub const BINARY_EXTENSION_HUGE_INSTANCE_COST: usize = 17070;

/// `Add256`: 2.32 GB.
pub const ADD_256_INSTANCE_COST: usize = 2376;

/// `ArithEq`: 2.12 GB.
pub const ARITH_EQ_INSTANCE_COST: usize = 2171;

/// `ArithEqLarge`: 16.97 GB.
pub const ARITH_EQ_LARGE_INSTANCE_COST: usize = 17377;

/// `Arith256X`: 1.56 GB.
pub const ARITH_256_X_INSTANCE_COST: usize = 1597;

/// `Arith256XLarge`: 6.23 GB.
pub const ARITH_256_X_LARGE_INSTANCE_COST: usize = 6380;

/// `ArithSecp256K1`: 1.84 GB.
pub const ARITH_SECP_256_K_1_INSTANCE_COST: usize = 1884;

/// `ArithSecp256K1Large`: 7.36 GB.
pub const ARITH_SECP_256_K_1_LARGE_INSTANCE_COST: usize = 7537;

/// `ArithBn254`: 1.93 GB.
pub const ARITH_BN_254_INSTANCE_COST: usize = 1976;

/// `ArithBn254Large`: 7.73 GB.
pub const ARITH_BN_254_LARGE_INSTANCE_COST: usize = 7916;

/// `ArithEq384`: 1.96 GB.
pub const ARITH_EQ_384_INSTANCE_COST: usize = 2007;

/// `ArithEq384Large`: 7.86 GB.
pub const ARITH_EQ_384_LARGE_INSTANCE_COST: usize = 8049;

/// `BabyJubJub`: 0.60 GB.
pub const BABY_JUB_JUB_INSTANCE_COST: usize = 614;

/// `Keccakf`: 12.54 GB.
pub const KECCAKF_INSTANCE_COST: usize = 12841;

/// `Sha256f`: 0.74 GB.
pub const SHA_256_F_INSTANCE_COST: usize = 758;

/// `Poseidon`: 0.86 GB.
pub const POSEIDON_INSTANCE_COST: usize = 881;

/// `Blake2br`: 1.10 GB.
pub const BLAKE_2_BR_INSTANCE_COST: usize = 1126;

/// `Blake3f`: 4.12 GB.
pub const BLAKE_3_F_INSTANCE_COST: usize = 4219;

/// `Dma`: 3.29 GB.
pub const DMA_INSTANCE_COST: usize = 3369;

/// `Dma64Aligned`: 3.79 GB.
pub const DMA_64_ALIGNED_INSTANCE_COST: usize = 3881;

/// `Dma64AlignedLarge`: 15.15 GB.
pub const DMA_64_ALIGNED_LARGE_INSTANCE_COST: usize = 15514;

/// `Dma64AlignedMemSet`: 2.48 GB.
pub const DMA_64_ALIGNED_MEM_SET_INSTANCE_COST: usize = 2540;

/// `Dma64AlignedMem`: 2.94 GB.
pub const DMA_64_ALIGNED_MEM_INSTANCE_COST: usize = 3011;

/// `Dma64AlignedMemLarge`: 5.89 GB.
pub const DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST: usize = 6031;

/// `Dma64AlignedMemCpy`: 3.48 GB.
pub const DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST: usize = 3564;

/// `DmaUnaligned`: 2.69 GB.
pub const DMA_UNALIGNED_INSTANCE_COST: usize = 2755;

/// `DmaPrePost`: 4.63 GB.
pub const DMA_PRE_POST_INSTANCE_COST: usize = 4741;

/// `JumpDest`: 3.40 GB.
pub const JUMP_DEST_INSTANCE_COST: usize = 3482;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pil_helpers::*;
    use proofman_common::trace::TraceRow;
    use proofman_fields::Goldilocks;

    /// Bytes one field element takes, which is what turns a committed cell count into memory.
    const BYTES_X_CELL: usize = 8;

    /// Each cost must cover at least the committed trace of the air it is named after — its rows
    /// times the width of its trace row, in MB. The prover holds that trace along with everything
    /// built from it (the extended-domain evaluations, the Merkle trees, the FRI folding steps), so
    /// a cost below it cannot be a real memory figure: it is stale, or was lowered past what the air
    /// is.
    ///
    /// This is a loose bound on purpose. The trace is a small share of the peak — the extended
    /// domain alone is a multiple of it — so passing this does not mean a cost is fresh, only that
    /// it has not gone obviously wrong. Refreshing them means re-reading `build/setup.log`.
    ///
    /// Pairing each constant with its trace here is also what pins the naming: a constant whose name
    /// no longer matches an air fails to compile rather than quietly pricing the wrong thing.
    #[test]
    fn the_costs_cover_the_committed_trace() {
        macro_rules! check {
            // `Rom` binds its own row type into the trace alias, so it cannot be instantiated with
            // `<()>` like the rest.
            (bound: $trace:ident : $row:ident : $cost:ident) => {
                check!(@assert $trace::<Goldilocks>::NUM_ROWS, $row, $cost, $trace);
            };
            (@assert $rows:expr, $row:ident, $cost:ident, $trace:ident) => {
                let trace_mb = $rows * $row::<Goldilocks>::ROW_SIZE * BYTES_X_CELL / (1024 * 1024);
                assert!(
                    $cost >= trace_mb,
                    "{} ({} MB) is below the {trace_mb} MB committed trace of {}, which the prover \
                     holds in full: the air grew and the cost was not refreshed",
                    stringify!($cost),
                    $cost,
                    stringify!($trace),
                );
            };
            ($( $trace:ident : $row:ident : $cost:ident ),+ $(,)?) => {$(
                let trace_mb =
                    $trace::<()>::NUM_ROWS * $row::<Goldilocks>::ROW_SIZE * BYTES_X_CELL
                        / (1024 * 1024);
                assert!(
                    $cost >= trace_mb,
                    "{} ({} MB) is below the {trace_mb} MB committed trace of {}, which the prover \
                     holds in full: the air grew and the cost was not refreshed",
                    stringify!($cost),
                    $cost,
                    stringify!($trace),
                );
            )+};
        }
        check!(bound: RomTrace: RomTraceRow: ROM_INSTANCE_COST);
        check!(
        MainTrace: MainTraceRow: MAIN_INSTANCE_COST,
                    MemTrace: MemTraceRow: MEM_INSTANCE_COST,
                    InputDataTrace: InputDataTraceRow: INPUT_DATA_INSTANCE_COST,
                    RomDataTrace: RomDataTraceRow: ROM_DATA_INSTANCE_COST,
                    MemAlignTrace: MemAlignTraceRow: MEM_ALIGN_INSTANCE_COST,
                    MemAlignLargeTrace: MemAlignLargeTraceRow: MEM_ALIGN_LARGE_INSTANCE_COST,
                    MemAlignByteTrace: MemAlignByteTraceRow: MEM_ALIGN_BYTE_INSTANCE_COST,
                    MemAlignReadByteTrace: MemAlignReadByteTraceRow: MEM_ALIGN_READ_BYTE_INSTANCE_COST,
                    MemAlignWriteByteTrace: MemAlignWriteByteTraceRow: MEM_ALIGN_WRITE_BYTE_INSTANCE_COST,
                    MemAlignByteLargeTrace: MemAlignByteLargeTraceRow: MEM_ALIGN_BYTE_LARGE_INSTANCE_COST,
                    MemAlignReadByteLargeTrace: MemAlignReadByteLargeTraceRow: MEM_ALIGN_READ_BYTE_LARGE_INSTANCE_COST,
                    ArithTrace: ArithTraceRow: ARITH_INSTANCE_COST,
                    BinaryTrace: BinaryTraceRow: BINARY_INSTANCE_COST,
                    BinaryLargeTrace: BinaryLargeTraceRow: BINARY_LARGE_INSTANCE_COST,
                    BinaryHugeTrace: BinaryHugeTraceRow: BINARY_HUGE_INSTANCE_COST,
                    BinaryAddTrace: BinaryAddTraceRow: BINARY_ADD_INSTANCE_COST,
                    BinaryAddLargeTrace: BinaryAddLargeTraceRow: BINARY_ADD_LARGE_INSTANCE_COST,
                    BinaryAddHugeTrace: BinaryAddHugeTraceRow: BINARY_ADD_HUGE_INSTANCE_COST,
                    BinaryAddHiTrace: BinaryAddHiTraceRow: BINARY_ADD_HI_INSTANCE_COST,
                    BinaryAddHiLargeTrace: BinaryAddHiLargeTraceRow: BINARY_ADD_HI_LARGE_INSTANCE_COST,
                    BinaryAddHiHugeTrace: BinaryAddHiHugeTraceRow: BINARY_ADD_HI_HUGE_INSTANCE_COST,
                    BinaryExtensionTrace: BinaryExtensionTraceRow: BINARY_EXTENSION_INSTANCE_COST,
                    BinaryExtensionLargeTrace: BinaryExtensionLargeTraceRow: BINARY_EXTENSION_LARGE_INSTANCE_COST,
                    BinaryExtensionHugeTrace: BinaryExtensionHugeTraceRow: BINARY_EXTENSION_HUGE_INSTANCE_COST,
                    Add256Trace: Add256TraceRow: ADD_256_INSTANCE_COST,
                    ArithEqTrace: ArithEqTraceRow: ARITH_EQ_INSTANCE_COST,
                    ArithEqLargeTrace: ArithEqLargeTraceRow: ARITH_EQ_LARGE_INSTANCE_COST,
                    Arith256XTrace: Arith256XTraceRow: ARITH_256_X_INSTANCE_COST,
                    Arith256XLargeTrace: Arith256XLargeTraceRow: ARITH_256_X_LARGE_INSTANCE_COST,
                    ArithSecp256K1Trace: ArithSecp256K1TraceRow: ARITH_SECP_256_K_1_INSTANCE_COST,
                    ArithSecp256K1LargeTrace: ArithSecp256K1LargeTraceRow: ARITH_SECP_256_K_1_LARGE_INSTANCE_COST,
                    ArithBn254Trace: ArithBn254TraceRow: ARITH_BN_254_INSTANCE_COST,
                    ArithBn254LargeTrace: ArithBn254LargeTraceRow: ARITH_BN_254_LARGE_INSTANCE_COST,
                    ArithEq384Trace: ArithEq384TraceRow: ARITH_EQ_384_INSTANCE_COST,
                    ArithEq384LargeTrace: ArithEq384LargeTraceRow: ARITH_EQ_384_LARGE_INSTANCE_COST,
                    BabyJubJubTrace: BabyJubJubTraceRow: BABY_JUB_JUB_INSTANCE_COST,
                    KeccakfTrace: KeccakfTraceRow: KECCAKF_INSTANCE_COST,
                    Sha256fTrace: Sha256fTraceRow: SHA_256_F_INSTANCE_COST,
                    PoseidonTrace: PoseidonTraceRow: POSEIDON_INSTANCE_COST,
                    Blake2brTrace: Blake2brTraceRow: BLAKE_2_BR_INSTANCE_COST,
                    Blake3fTrace: Blake3fTraceRow: BLAKE_3_F_INSTANCE_COST,
                    DmaTrace: DmaTraceRow: DMA_INSTANCE_COST,
                    Dma64AlignedTrace: Dma64AlignedTraceRow: DMA_64_ALIGNED_INSTANCE_COST,
                    Dma64AlignedLargeTrace: Dma64AlignedLargeTraceRow: DMA_64_ALIGNED_LARGE_INSTANCE_COST,
                    Dma64AlignedMemSetTrace: Dma64AlignedMemSetTraceRow: DMA_64_ALIGNED_MEM_SET_INSTANCE_COST,
                    Dma64AlignedMemTrace: Dma64AlignedMemTraceRow: DMA_64_ALIGNED_MEM_INSTANCE_COST,
                    Dma64AlignedMemLargeTrace: Dma64AlignedMemLargeTraceRow: DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST,
                    Dma64AlignedMemCpyTrace: Dma64AlignedMemCpyTraceRow: DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST,
                    DmaUnalignedTrace: DmaUnalignedTraceRow: DMA_UNALIGNED_INSTANCE_COST,
                    DmaPrePostTrace: DmaPrePostTraceRow: DMA_PRE_POST_INSTANCE_COST,
                    JumpDestTrace: JumpDestTraceRow: JUMP_DEST_INSTANCE_COST,
                );
    }
}
