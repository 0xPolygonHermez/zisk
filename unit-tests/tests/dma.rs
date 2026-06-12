//! Per-AIR unit tests for the DMA precompile family. Four input types drive
//! twelve SMs; the `encoded` field (built with the `DmaInfo::encode_*`
//! helpers) packs the pre/loop/post split, offsets and fill byte, and must be
//! consistent with the src/dst addresses and data supplied.

use precompiles_helpers::DmaInfo;
use zisk_core::zisk_ops::ZiskOp;
use zisk_prover_backend::{
    inputs::{Dma64AlignedInput, DmaInput, DmaPrePostInput, DmaUnalignedInput},
    testing::with_prover,
    Dma64AlignedInputCpySm, Dma64AlignedMemCpySm, Dma64AlignedMemSetSm, Dma64AlignedMemSm,
    Dma64AlignedSm, DmaMemCpySm, DmaPrePostInputCpySm, DmaPrePostMemCpySm, DmaPrePostSm, DmaSm,
    DmaUnalignedSm,
};

const SRC: u32 = 0xa000_0000;
const DST: u32 = 0xa001_0000;

/// Fully aligned 8-byte memcpy: no pre/post phases, one loop op.
fn aligned_memcpy() -> DmaInput {
    DmaInput {
        src: SRC,
        dst: DST,
        op: ZiskOp::DMA_MEMCPY,
        encoded: DmaInfo::encode_memcpy(0, 0, 8),
        count_bus: 0,
        step: 1,
    }
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_honest_input_verifies() {
    with_prover(|prover| {
        let result =
            prover.input::<DmaSm>(aligned_memcpy()).run().expect("verification run failed");
        assert!(result.valid, "honest Dma input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_memcpy_honest_input_verifies() {
    with_prover(|prover| {
        let result =
            prover.input::<DmaMemCpySm>(aligned_memcpy()).run().expect("verification run failed");
        assert!(result.valid, "honest DmaMemCpy input should satisfy all constraints");
    });
}

/// Pre phase of an unaligned-destination memcpy: dst offset 5, so 3 bytes
/// (positions 5..7) are read-modify-written into the destination word.
fn pre_phase_memcpy() -> DmaPrePostInput {
    DmaPrePostInput {
        src: SRC,
        dst: DST,
        step: 1,
        encoded: DmaInfo::encode_memcpy(5, 0, 8),
        src_values: [0x0102_0304_0506_0708, 0],
        dst_pre_value: 0xDEAD_BEEF_CAFE_BABE,
        op: ZiskOp::DMA_MEMCPY,
    }
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_pre_post_honest_input_verifies() {
    with_prover(|prover| {
        let result =
            prover.input::<DmaPrePostSm>(pre_phase_memcpy()).run().expect("verification failed");
        assert!(result.valid, "honest DmaPrePost input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_pre_post_memcpy_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<DmaPrePostMemCpySm>(pre_phase_memcpy())
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest DmaPrePostMemCpy input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_pre_post_inputcpy_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<DmaPrePostInputCpySm>(DmaPrePostInput {
                src: 0,
                dst: DST,
                step: 1,
                encoded: DmaInfo::encode_inputcpy(5, 8),
                src_values: [0x0102_0304_0506_0708, 0],
                dst_pre_value: 0xDEAD_BEEF_CAFE_BABE,
                op: ZiskOp::DMA_INPUTCPY,
            })
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest DmaPrePostInputCpy input should satisfy all constraints");
    });
}

/// Aligned 8-byte memcpy loop phase: one 64-bit op, one row.
fn aligned_loop_memcpy() -> Dma64AlignedInput {
    Dma64AlignedInput {
        src: SRC,
        dst: DST,
        is_last_instance_input: true,
        op: ZiskOp::DMA_MEMCPY,
        trace_offset: 0,
        skip_rows: 0,
        rows: 1,
        step: 1,
        encoded: DmaInfo::encode_memcpy(0, 0, 8),
        src_values: vec![0x0102_0304_0506_0708],
    }
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_64_aligned_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Dma64AlignedSm>(aligned_loop_memcpy())
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest Dma64Aligned input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_64_aligned_memcpy_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Dma64AlignedMemCpySm>(aligned_loop_memcpy())
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest Dma64AlignedMemCpy input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_64_aligned_mem_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Dma64AlignedMemSm>(aligned_loop_memcpy())
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest Dma64AlignedMem input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_64_aligned_memset_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Dma64AlignedMemSetSm>(Dma64AlignedInput {
                src: 0, // memset has no source
                dst: DST,
                is_last_instance_input: true,
                op: ZiskOp::DMA_XMEMSET,
                trace_offset: 0,
                skip_rows: 0,
                rows: 1,
                step: 1,
                encoded: DmaInfo::encode_memset(0, 16, 0xAA),
                src_values: vec![],
            })
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest Dma64AlignedMemSet input should satisfy all constraints");
    });
}

#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_64_aligned_inputcpy_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<Dma64AlignedInputCpySm>(Dma64AlignedInput {
                src: 0,
                dst: DST,
                is_last_instance_input: true,
                op: ZiskOp::DMA_INPUTCPY,
                trace_offset: 0,
                skip_rows: 0,
                rows: 1,
                step: 1,
                encoded: DmaInfo::encode_inputcpy(0, 16),
                src_values: vec![0x0102_0304_0506_0708, 0x1112_1314_1516_1718],
            })
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest Dma64AlignedInputCpy input should satisfy all constraints");
    });
}

/// Unaligned memcpy (dst offset 3, src offset 0, 16 bytes): pre 5 bytes,
/// one 8-byte loop op, post 3 bytes. The unaligned loop needs `loop_count + 1`
/// source words for the boundary read.
#[test]
#[ignore = "requires ~/.zisk/provingKey"]
fn dma_unaligned_honest_input_verifies() {
    with_prover(|prover| {
        let result = prover
            .input::<DmaUnalignedSm>(DmaUnalignedInput {
                src: SRC,
                dst: DST + 3,
                is_last_instance_input: true,
                is_mem_eq: false,
                trace_offset: 0,
                skip: 0,
                count: 2,
                step: 1,
                encoded: DmaInfo::encode_memcpy(3, 0, 16),
                src_values: vec![0x0102_0304_0506_0708, 0x1112_1314_1516_1718],
            })
            .run()
            .expect("verification failed");
        assert!(result.valid, "honest DmaUnaligned input should satisfy all constraints");
    });
}
