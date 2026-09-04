//! What one instance of each air costs, so the planners can price a layout.
//!
//! One constant per air. The value is the whole instance — `rows x columns` — regardless of how full
//! it ends up: an instance is a full trace either way.
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
//! `columns` is what the setup commits, which is the figure the executor prices instances with at
//! run time — the sum of `stark_info.map_sections_n` over every section but `const` (see
//! `setup_cost` in `executor/src/adapters.rs`). The PIL compiler reports its two halves per air as
//! `witness: <stage1>,<stage2>` (see `build/compile.log`); stage-2 columns live in the cubic
//! extension, so each costs three field elements, and the quotient section adds six more:
//!
//! ```text
//! columns = stage1 + 3 x stage2 + 6
//! ```
//!
//! That formula reproduces exactly the six weights that used to be declared by hand in the planners
//! (`Binary` 60, `BinaryAdd` 25, `BinaryAddHi` 36, `BinaryExtension` 58, `MemAlignReadByte` 25,
//! `MemAlignWriteByte` 32), which is why it is trusted for the rest.
//!
//! To regenerate after a PIL change, read the `witness:` line of each air from a fresh compilation
//! and multiply by that air's `NUM_ROWS`. [`the_costs_cover_the_committed_trace`] catches the
//! coarsest form of staleness — a cost that no longer even covers the air's committed area — but it
//! cannot see stage-2 growth, so an air that grows only those will not be flagged.

/// `Main`: 2^23 rows of 68 columns (38 + 3x8 + 6).
pub const MAIN_INSTANCE_COST: usize = 570_425_344;

/// `Rom`: 2^22 rows of 16 columns (1 + 3x3 + 6).
pub const ROM_INSTANCE_COST: usize = 67_108_864;

/// `Mem`: 2^23 rows of 28 columns (13 + 3x3 + 6).
pub const MEM_INSTANCE_COST: usize = 234_881_024;

/// `InputData`: 2^22 rows of 27 columns (9 + 3x4 + 6).
pub const INPUT_DATA_INSTANCE_COST: usize = 113_246_208;

/// `RomData`: 2^22 rows of 14 columns (5 + 3x1 + 6).
pub const ROM_DATA_INSTANCE_COST: usize = 58_720_256;

/// `MemAlign`: 2^21 rows of 50 columns (32 + 3x4 + 6).
pub const MEM_ALIGN_INSTANCE_COST: usize = 104_857_600;

/// `MemAlignLarge`: 2^23 rows of 50 columns (32 + 3x4 + 6).
pub const MEM_ALIGN_LARGE_INSTANCE_COST: usize = 419_430_400;

/// `MemAlignByte`: 2^22 rows of 34 columns (16 + 3x4 + 6).
pub const MEM_ALIGN_BYTE_INSTANCE_COST: usize = 142_606_336;

/// `MemAlignReadByte`: 2^22 rows of 25 columns (10 + 3x3 + 6).
pub const MEM_ALIGN_READ_BYTE_INSTANCE_COST: usize = 104_857_600;

/// `MemAlignWriteByte`: 2^22 rows of 32 columns (14 + 3x4 + 6).
pub const MEM_ALIGN_WRITE_BYTE_INSTANCE_COST: usize = 134_217_728;

/// `MemAlignByteLarge`: 2^23 rows of 34 columns (16 + 3x4 + 6).
pub const MEM_ALIGN_BYTE_LARGE_INSTANCE_COST: usize = 285_212_672;

/// `MemAlignReadByteLarge`: 2^23 rows of 25 columns (10 + 3x3 + 6).
pub const MEM_ALIGN_READ_BYTE_LARGE_INSTANCE_COST: usize = 209_715_200;

/// `Arith`: 2^21 rows of 96 columns (45 + 3x15 + 6).
pub const ARITH_INSTANCE_COST: usize = 201_326_592;

/// `Binary`: 2^22 rows of 60 columns (39 + 3x5 + 6).
pub const BINARY_INSTANCE_COST: usize = 251_658_240;

/// `BinaryLarge`: 2^23 rows of 60 columns (39 + 3x5 + 6).
pub const BINARY_LARGE_INSTANCE_COST: usize = 503_316_480;

/// `BinaryAdd`: 2^22 rows of 25 columns (10 + 3x3 + 6).
pub const BINARY_ADD_INSTANCE_COST: usize = 104_857_600;

/// `BinaryAddLarge`: 2^23 rows of 25 columns (10 + 3x3 + 6).
pub const BINARY_ADD_LARGE_INSTANCE_COST: usize = 209_715_200;

/// `BinaryAddHi`: 2^22 rows of 36 columns (15 + 3x5 + 6).
pub const BINARY_ADD_HI_INSTANCE_COST: usize = 150_994_944;

/// `BinaryAddHiLarge`: 2^22 rows of 55 columns (25 + 3x8 + 6).
pub const BINARY_ADD_HI_LARGE_INSTANCE_COST: usize = 230_686_720;

/// `BinaryExtension`: 2^22 rows of 58 columns (34 + 3x6 + 6).
pub const BINARY_EXTENSION_INSTANCE_COST: usize = 243_269_632;

/// `BinaryExtensionLarge`: 2^23 rows of 58 columns (34 + 3x6 + 6).
pub const BINARY_EXTENSION_LARGE_INSTANCE_COST: usize = 486_539_264;

/// `Add256`: 2^20 rows of 104 columns (47 + 3x17 + 6).
pub const ADD_256_INSTANCE_COST: usize = 109_051_904;

/// `ArithEq`: 2^20 rows of 87 columns (45 + 3x12 + 6).
pub const ARITH_EQ_INSTANCE_COST: usize = 91_226_112;

/// `ArithEqLarge`: 2^23 rows of 87 columns (45 + 3x12 + 6).
pub const ARITH_EQ_LARGE_INSTANCE_COST: usize = 729_808_896;

/// `Arith256X`: 2^20 rows of 52 columns (19 + 3x9 + 6).
pub const ARITH_256_X_INSTANCE_COST: usize = 54_525_952;

/// `Arith256XLarge`: 2^22 rows of 52 columns (19 + 3x9 + 6).
pub const ARITH_256_X_LARGE_INSTANCE_COST: usize = 218_103_808;

/// `ArithSecp256K1`: 2^20 rows of 69 columns (27 + 3x12 + 6).
pub const ARITH_SECP_256_K_1_INSTANCE_COST: usize = 72_351_744;

/// `ArithSecp256K1Large`: 2^22 rows of 69 columns (27 + 3x12 + 6).
pub const ARITH_SECP_256_K_1_LARGE_INSTANCE_COST: usize = 289_406_976;

/// `ArithBn254`: 2^20 rows of 75 columns (33 + 3x12 + 6).
pub const ARITH_BN_254_INSTANCE_COST: usize = 78_643_200;

/// `ArithBn254Large`: 2^22 rows of 75 columns (33 + 3x12 + 6).
pub const ARITH_BN_254_LARGE_INSTANCE_COST: usize = 314_572_800;

/// `ArithEq384`: 2^20 rows of 77 columns (35 + 3x12 + 6).
pub const ARITH_EQ_384_INSTANCE_COST: usize = 80_740_352;

/// `ArithEq384Large`: 2^22 rows of 77 columns (35 + 3x12 + 6).
pub const ARITH_EQ_384_LARGE_INSTANCE_COST: usize = 322_961_408;

/// `BabyJubJub`: 2^18 rows of 105 columns (39 + 3x20 + 6).
pub const BABY_JUB_JUB_INSTANCE_COST: usize = 27_525_120;

/// `Keccakf`: 2^20 rows of 642 columns (453 + 3x61 + 6).
pub const KECCAKF_INSTANCE_COST: usize = 673_185_792;

/// `Sha256f`: 2^18 rows of 117 columns (102 + 3x3 + 6).
pub const SHA_256_F_INSTANCE_COST: usize = 30_670_848;

/// `Poseidon`: 2^17 rows of 201 columns (84 + 3x37 + 6).
pub const POSEIDON_INSTANCE_COST: usize = 26_345_472;

/// `Blake2br`: 2^18 rows of 227 columns (119 + 3x34 + 6).
pub const BLAKE_2_BR_INSTANCE_COST: usize = 59_506_688;

/// `Blake3f`: 2^20 rows of 216 columns (114 + 3x32 + 6).
pub const BLAKE_3_F_INSTANCE_COST: usize = 226_492_416;

/// `Dma`: 2^21 rows of 61 columns (34 + 3x7 + 6).
pub const DMA_INSTANCE_COST: usize = 127_926_272;

/// `Dma64Aligned`: 2^21 rows of 77 columns (35 + 3x12 + 6).
pub const DMA_64_ALIGNED_INSTANCE_COST: usize = 161_480_704;

/// `Dma64AlignedLarge`: 2^23 rows of 77 columns (35 + 3x12 + 6).
pub const DMA_64_ALIGNED_LARGE_INSTANCE_COST: usize = 645_922_816;

/// `Dma64AlignedMemSet`: 2^21 rows of 35 columns (14 + 3x5 + 6).
pub const DMA_64_ALIGNED_MEM_SET_INSTANCE_COST: usize = 73_400_320;

/// `Dma64AlignedMem`: 2^21 rows of 50 columns (26 + 3x6 + 6).
pub const DMA_64_ALIGNED_MEM_INSTANCE_COST: usize = 104_857_600;

/// `Dma64AlignedMemLarge`: 2^22 rows of 50 columns (26 + 3x6 + 6).
pub const DMA_64_ALIGNED_MEM_LARGE_INSTANCE_COST: usize = 209_715_200;

/// `Dma64AlignedMemCpy`: 2^21 rows of 67 columns (31 + 3x10 + 6).
pub const DMA_64_ALIGNED_MEM_CPY_INSTANCE_COST: usize = 140_509_184;

/// `DmaUnaligned`: 2^21 rows of 42 columns (24 + 3x4 + 6).
pub const DMA_UNALIGNED_INSTANCE_COST: usize = 88_080_384;

/// `DmaPrePost`: 2^21 rows of 102 columns (66 + 3x10 + 6).
pub const DMA_PRE_POST_INSTANCE_COST: usize = 213_909_504;

/// `JumpDest`: 2^21 rows of 62 columns (32 + 3x8 + 6).
pub const JUMP_DEST_INSTANCE_COST: usize = 130_023_424;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pil_helpers::*;
    use proofman_common::trace::TraceRow;
    use proofman_fields::Goldilocks;

    /// Each cost must cover at least the committed area of the air it is named after — its rows
    /// times the width of its trace row — since the setup counts those columns plus the stage-2 and
    /// quotient ones. A cost that no longer does is stale, or was lowered past what the air is.
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
                let committed = $rows * $row::<Goldilocks>::ROW_SIZE;
                assert!(
                    $cost >= committed,
                    "{} ({}) no longer covers the {committed} committed cells of {}: the air grew \
                     and the cost was not refreshed",
                    stringify!($cost),
                    $cost,
                    stringify!($trace),
                );
            };
            ($( $trace:ident : $row:ident : $cost:ident ),+ $(,)?) => {$(
                let committed = $trace::<()>::NUM_ROWS * $row::<Goldilocks>::ROW_SIZE;
                assert!(
                    $cost >= committed,
                    "{} ({}) no longer covers the {committed} committed cells of {}: the air grew \
                     and the cost was not refreshed",
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
                    BinaryAddTrace: BinaryAddTraceRow: BINARY_ADD_INSTANCE_COST,
                    BinaryAddLargeTrace: BinaryAddLargeTraceRow: BINARY_ADD_LARGE_INSTANCE_COST,
                    BinaryAddHiTrace: BinaryAddHiTraceRow: BINARY_ADD_HI_INSTANCE_COST,
                    BinaryAddHiLargeTrace: BinaryAddHiLargeTraceRow: BINARY_ADD_HI_LARGE_INSTANCE_COST,
                    BinaryExtensionTrace: BinaryExtensionTraceRow: BINARY_EXTENSION_INSTANCE_COST,
                    BinaryExtensionLargeTrace: BinaryExtensionLargeTraceRow: BINARY_EXTENSION_LARGE_INSTANCE_COST,
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
