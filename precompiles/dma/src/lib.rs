mod dma;
mod dma_64_aligned;
mod dma_bus_device;
mod dma_checkpoint;
mod dma_collect_counters;
mod dma_collector_routing_log;
mod dma_common;
mod dma_constants;
mod dma_gen_inputcpy_mem_inputs;
mod dma_gen_mem_inputs;
mod dma_gen_memcmp_mem_inputs;
mod dma_gen_memcpy_mem_inputs;
mod dma_gen_memset_mem_inputs;
mod dma_instance_info;
mod dma_instances_builder;
mod dma_manager;
mod dma_planner;
mod dma_pre_post;
mod dma_strategy;
mod dma_unaligned;

pub use dma::*;
pub use dma_64_aligned::*;
pub use dma_bus_device::*;
pub use dma_checkpoint::*;
pub use dma_collect_counters::*;
pub use dma_collector_routing_log::*;
pub use dma_common::*;
pub use dma_constants::*;
pub use dma_gen_inputcpy_mem_inputs::*;
pub use dma_gen_mem_inputs::*;
pub use dma_gen_memcmp_mem_inputs::*;
pub use dma_gen_memcpy_mem_inputs::*;
pub use dma_gen_memset_mem_inputs::*;
pub use dma_instance_info::*;
pub use dma_instances_builder::*;
pub use dma_manager::*;
pub use dma_planner::*;
pub use dma_pre_post::*;
pub use dma_strategy::*;
pub use dma_unaligned::*;

// =====================================================================
// Unit-test framework markers — 12 AIR ids partitioned across 4 module
// trait families. Each marker's `manager` is its own inner SM (the
// `DmaManager` orchestrator only exists to bundle them at construction
// time; the executor's manager registry extracts each inner SM).
// =====================================================================

use zisk_common::{unit_test_sm, SegmentId};
use zisk_pil::{
    Dma64AlignedInputCpyTrace, Dma64AlignedInputCpyTraceRow, Dma64AlignedMemCpyTrace,
    Dma64AlignedMemCpyTraceRow, Dma64AlignedMemSetTrace, Dma64AlignedMemSetTraceRow,
    Dma64AlignedMemTrace, Dma64AlignedMemTraceRow, Dma64AlignedTrace, Dma64AlignedTraceRow,
    DmaInputCpyTrace, DmaInputCpyTraceRow, DmaMemCpyTrace, DmaMemCpyTraceRow,
    DmaPrePostInputCpyTrace, DmaPrePostInputCpyTraceRow, DmaPrePostMemCpyTrace,
    DmaPrePostMemCpyTraceRow, DmaPrePostTrace, DmaPrePostTraceRow, DmaTrace, DmaTraceRow,
    DmaUnalignedTrace, DmaUnalignedTraceRow, DmaUnalignedTraceRowPacked, DMA_64_ALIGNED_AIR_IDS,
    DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS, DMA_64_ALIGNED_MEM_CPY_AIR_IDS,
    DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS, DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS,
    DMA_PRE_POST_AIR_IDS, DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS,
    DMA_UNALIGNED_AIR_IDS,
};

const DEFAULT_SEGMENT_ID: SegmentId = SegmentId(0);
const DEFAULT_LAST_SEGMENT: bool = true;

// DmaModule family ----------------------------------------------------------

unit_test_sm! {
    DmaSm => {
        name: "Dma",
        air: DMA_AIR_IDS[0],
        input: DmaInput,
        row: DmaTraceRow<F>,
        manager: DmaSM<F>,
        trace: DmaTrace,
        chunk_size: |_| DmaTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], buf, packed)
        },
    }
}

unit_test_sm! {
    DmaMemCpySm => {
        name: "DmaMemCpy",
        air: DMA_MEM_CPY_AIR_IDS[0],
        input: DmaInput,
        row: DmaMemCpyTraceRow<F>,
        manager: DmaMemCpySM<F>,
        trace: DmaMemCpyTrace,
        chunk_size: |_| DmaMemCpyTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], buf, packed)
        },
    }
}

unit_test_sm! {
    DmaInputCpySm => {
        name: "DmaInputCpy",
        air: DMA_INPUT_CPY_AIR_IDS[0],
        input: DmaInput,
        row: DmaInputCpyTraceRow<F>,
        manager: DmaInputCpySM<F>,
        trace: DmaInputCpyTrace,
        chunk_size: |_| DmaInputCpyTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], buf, packed)
        },
    }
}

// DmaPrePostModule family ---------------------------------------------------

unit_test_sm! {
    DmaPrePostSm => {
        name: "DmaPrePost",
        air: DMA_PRE_POST_AIR_IDS[0],
        input: DmaPrePostInput,
        row: DmaPrePostTraceRow<F>,
        manager: DmaPrePostSM<F>,
        trace: DmaPrePostTrace,
        chunk_size: |_| DmaPrePostTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], buf, packed)
        },
    }
}

unit_test_sm! {
    DmaPrePostMemCpySm => {
        name: "DmaPrePostMemCpy",
        air: DMA_PRE_POST_MEM_CPY_AIR_IDS[0],
        input: DmaPrePostInput,
        row: DmaPrePostMemCpyTraceRow<F>,
        manager: DmaPrePostMemCpySM<F>,
        trace: DmaPrePostMemCpyTrace,
        chunk_size: |_| DmaPrePostMemCpyTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], buf, packed)
        },
    }
}

unit_test_sm! {
    DmaPrePostInputCpySm => {
        name: "DmaPrePostInputCpy",
        air: DMA_PRE_POST_INPUT_CPY_AIR_IDS[0],
        input: DmaPrePostInput,
        row: DmaPrePostInputCpyTraceRow<F>,
        manager: DmaPrePostInputCpySM<F>,
        trace: DmaPrePostInputCpyTrace,
        chunk_size: |_| DmaPrePostInputCpyTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], buf, packed)
        },
    }
}

// Dma64AlignedModule family -------------------------------------------------

unit_test_sm! {
    Dma64AlignedSm => {
        name: "Dma64Aligned",
        air: DMA_64_ALIGNED_AIR_IDS[0],
        input: Dma64AlignedInput,
        row: Dma64AlignedTraceRow<F>,
        manager: Dma64AlignedSM<F>,
        trace: Dma64AlignedTrace,
        chunk_size: |_| Dma64AlignedTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf, packed)
        },
    }
}

unit_test_sm! {
    Dma64AlignedMemCpySm => {
        name: "Dma64AlignedMemCpy",
        air: DMA_64_ALIGNED_MEM_CPY_AIR_IDS[0],
        input: Dma64AlignedInput,
        row: Dma64AlignedMemCpyTraceRow<F>,
        manager: Dma64AlignedMemCpySM<F>,
        trace: Dma64AlignedMemCpyTrace,
        chunk_size: |_| Dma64AlignedMemCpyTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf, packed)
        },
    }
}

unit_test_sm! {
    Dma64AlignedInputCpySm => {
        name: "Dma64AlignedInputCpy",
        air: DMA_64_ALIGNED_INPUT_CPY_AIR_IDS[0],
        input: Dma64AlignedInput,
        row: Dma64AlignedInputCpyTraceRow<F>,
        manager: Dma64AlignedInputCpySM<F>,
        trace: Dma64AlignedInputCpyTrace,
        chunk_size: |_| Dma64AlignedInputCpyTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf, packed)
        },
    }
}

unit_test_sm! {
    Dma64AlignedMemSetSm => {
        name: "Dma64AlignedMemSet",
        air: DMA_64_ALIGNED_MEM_SET_AIR_IDS[0],
        input: Dma64AlignedInput,
        row: Dma64AlignedMemSetTraceRow<F>,
        manager: Dma64AlignedMemSetSM<F>,
        trace: Dma64AlignedMemSetTrace,
        chunk_size: |_| Dma64AlignedMemSetTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf, packed)
        },
    }
}

unit_test_sm! {
    Dma64AlignedMemSm => {
        name: "Dma64AlignedMem",
        air: DMA_64_ALIGNED_MEM_AIR_IDS[0],
        input: Dma64AlignedInput,
        row: Dma64AlignedMemTraceRow<F>,
        manager: Dma64AlignedMemSM<F>,
        trace: Dma64AlignedMemTrace,
        chunk_size: |_| Dma64AlignedMemTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            sm.compute_witness(&[inputs], DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf, packed)
        },
    }
}

// DmaUnaligned --------------------------------------------------------------

unit_test_sm! {
    DmaUnalignedSm => {
        name: "DmaUnaligned",
        air: DMA_UNALIGNED_AIR_IDS[0],
        input: DmaUnalignedInput,
        row: DmaUnalignedTraceRow<F>,
        manager: DmaUnalignedSM<F>,
        trace: DmaUnalignedTrace,
        chunk_size: |_| DmaUnalignedTrace::<usize>::NUM_ROWS,
        compute: |sm, _sctx, inputs, buf, packed| {
            let inputs = vec![inputs];
            if packed {
                sm.compute_witness::<DmaUnalignedTraceRowPacked<F>>(
                    &inputs, DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf,
                )
            } else {
                sm.compute_witness::<DmaUnalignedTraceRow<F>>(
                    &inputs, DEFAULT_SEGMENT_ID, DEFAULT_LAST_SEGMENT, buf,
                )
            }
        },
    }
}
