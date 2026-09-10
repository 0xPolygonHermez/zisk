use zisk_common::OPERATION_PRECOMPILED_BUS_DATA_SIZE;

pub const PARAMS: usize = 4;
pub const READ_PARAMS: usize = 2;
pub const DIRECT_READ_PARAMS: usize = 1;
pub const WRITE_PARAMS: usize = 1;
pub const RESULT_PARAMS: usize = 1;
pub const PARAM_CHUNKS: usize = 4;
pub const START_READ_PARAMS: usize = OPERATION_PRECOMPILED_BUS_DATA_SIZE + PARAMS;
pub const START_WRITE_PARAMS: usize =
    START_READ_PARAMS + READ_PARAMS * PARAM_CHUNKS + RESULT_PARAMS;
pub const WRITE_ADDR_PARAM: usize = READ_PARAMS + DIRECT_READ_PARAMS;
pub const DMA_64_ALIGNED_OPS_BY_ROW: usize = 4;
pub const DMA_64_ALIGNED_INPUTCPY_OPS_BY_ROW: usize = 4;
pub const DMA_64_ALIGNED_MEMCPY_OPS_BY_ROW: usize = 4;
pub const DMA_64_ALIGNED_MEMSET_OPS_BY_ROW: usize = 4;
pub const DMA_64_ALIGNED_MEM_OPS_BY_ROW: usize = 4;

/// Words of a sequence proved on each `DmaUnaligned` row, i.e. `op_x_row` in `dma_unaligned.pil`.
///
/// MUST match the PIL: `DmaUnalignedTraceRow::read_bytes` is an array of exactly this many lanes,
/// which is what `tests::the_ops_by_row_matches_the_generated_row` checks.
pub const DMA_UNALIGNED_OPS_BY_ROW: usize = 4;
pub const DMA_ROM_WITHOUT_MEMCMP_SIZE: usize = 1 << 15;
pub const DMA_ROM_WITH_MEMCMP_SIZE: usize = 3 * (1 << 15);

#[cfg(test)]
mod tests {
    use super::*;

    /// The packing constants name a shape the PIL decides, so they are checked against the row it
    /// generated rather than taken on trust: a change in `zisk.pil` that is not mirrored here fails
    /// this test instead of corrupting a trace.
    #[test]
    fn the_ops_by_row_match_the_generated_rows() {
        use proofman_fields::Goldilocks;
        use zisk_pil::*;

        // `sel_op_from_1` carries every lane but the first, which is always selected — so the row
        // packs one more operation than that array is long. It only exists while the air packs more
        // than one, which every air here does.
        macro_rules! check {
            ($row:ident, $konst:path) => {
                assert_eq!(
                    $row::<Goldilocks>::default().get_all_sel_op_from_1().len() + 1,
                    $konst,
                    concat!(stringify!($konst), " does not match ", stringify!($row)),
                );
            };
        }

        check!(Dma64AlignedTraceRow, DMA_64_ALIGNED_OPS_BY_ROW);
        check!(Dma64AlignedLargeTraceRow, DMA_64_ALIGNED_OPS_BY_ROW);
        check!(Dma64AlignedMemTraceRow, DMA_64_ALIGNED_MEM_OPS_BY_ROW);
        check!(Dma64AlignedMemLargeTraceRow, DMA_64_ALIGNED_MEM_OPS_BY_ROW);
        check!(Dma64AlignedMemCpyTraceRow, DMA_64_ALIGNED_MEMCPY_OPS_BY_ROW);
        check!(Dma64AlignedMemSetTraceRow, DMA_64_ALIGNED_MEMSET_OPS_BY_ROW);
        check!(DmaUnalignedTraceRow, DMA_UNALIGNED_OPS_BY_ROW);

        assert_eq!(
            DmaUnalignedTraceRow::<Goldilocks>::default().get_all_read_bytes().len(),
            DMA_UNALIGNED_OPS_BY_ROW,
            "DMA_UNALIGNED_OPS_BY_ROW does not match DmaUnalignedTraceRow",
        );
    }
}
