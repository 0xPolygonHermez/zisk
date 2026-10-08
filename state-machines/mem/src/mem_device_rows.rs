//! Which memory airs' rows this block takes from the GPU planner, and the MemAlign instances'
//! device path (`ZISK_MEM_GPU_FILL`, see `mem_gpu_fill`).

use crate::mem_gpu_fill::{compare_rows_masked, gpu_fill_mode, packed_used_bits, GpuFillMode};

fn family_bit(name: &str) -> u32 {
    match name {
        "ram" => zisk_common::MEM_ROWS_RAM,
        "rom" => zisk_common::MEM_ROWS_ROM,
        "input" => zisk_common::MEM_ROWS_INPUT,
        "align" => zisk_common::MEM_ROWS_ALIGN,
        _ => 0,
    }
}

/// Whether this block's rows of the memory air family `name` ("ram", "rom", "input", "align") come
/// from the GPU planner: arena or slot mode and a device fill that succeeded for the block (the
/// rows still on the device, or already in the planner's host memory for the proofs); otherwise
/// the instances are collected and filled on the CPU.
pub(crate) fn rows_on_device(name: &str) -> bool {
    use std::sync::atomic::Ordering::Acquire;
    let bit = family_bit(name);
    matches!(gpu_fill_mode(), GpuFillMode::Arena | GpuFillMode::Slot)
        && (zisk_common::MEM_ROWS_ON_DEVICE.load(Acquire)
            | zisk_common::MEM_ROWS_ON_HOST.load(Acquire))
            & bit
            != 0
}

/// `ZISK_MEM_GPU_FILL=slot` while the family's rows are still on the device: the instance is
/// committed by the prover's kernel. Once the arena went back, the rows come from the host copy.
pub(crate) fn slot_pending(name: &str) -> bool {
    gpu_fill_mode() == GpuFillMode::Slot
        && zisk_common::MEM_ROWS_ON_DEVICE.load(std::sync::atomic::Ordering::Acquire)
            & family_bit(name)
            != 0
}

/// The kernel-input family of the MemAlign airs (the other three live with the staging helpers).
pub(crate) const SLOT_FAMILY_ALIGN: u32 = 3;

/// The prover's declaration of the memory airs the planner's kernel fills (`ZISK_MEM_GPU_FILL=slot`):
/// Mem, RomData, InputData and the seven MemAlign airs, packed rows, host traces allowed (the
/// proofs collect them on the CPU).
pub fn mem_slot_witness_airs() -> Vec<proofman_common::GpuWitnessAir> {
    use proofman_common::{GpuWitnessAir, TraceLayout};
    use zisk_pil::*;
    let op_bytes = std::mem::size_of::<crate::mem_gpu_fill::MemSlotOp>() as u64;
    [
        MemTrace::<()>::AIR_ID,
        RomDataTrace::<()>::AIR_ID,
        InputDataTrace::<()>::AIR_ID,
        MemAlignTrace::<()>::AIR_ID,
        MemAlignLargeTrace::<()>::AIR_ID,
        MemAlignByteTrace::<()>::AIR_ID,
        MemAlignByteLargeTrace::<()>::AIR_ID,
        MemAlignReadByteTrace::<()>::AIR_ID,
        MemAlignReadByteLargeTrace::<()>::AIR_ID,
        MemAlignWriteByteTrace::<()>::AIR_ID,
    ]
    .into_iter()
    .map(|air_id| {
        GpuWitnessAir::new(
            ZISK_AIRGROUP_ID,
            air_id,
            op_bytes,
            op_bytes,
            TraceLayout::PackedCm1,
            zisk_sm_mem_planner::zisk_mem_witness_slot_kernel,
        )
        .with_host_trace()
        .with_planner_gpu()
    })
    .collect()
}

/// The rows of MemAlign instance (`air_id`, `segment`) from the GPU planner into `rows`; the rows
/// its accesses use.
pub(crate) fn align_device_rows(
    air_id: usize,
    segment: usize,
    rows: &mut [u64],
) -> proofman_common::ProofmanResult<usize> {
    zisk_sm_mem_planner::gpu_align_witness_fill(air_id, segment, rows).map_err(|e| {
        proofman_common::ProofmanError::InvalidParameters(format!(
            "MemAlign air {air_id}[{segment}] witness from the GPU planner failed: {e}"
        ))
    })
}

/// After a CPU fill of a MemAlign instance: the arena check against the device rows when
/// `ZISK_MEM_GPU_FILL=arena-check`, and the rows' hash when `ZISK_MEM_TRACE_HASH_DIR` is set.
pub(crate) fn align_filled(
    air_id: usize,
    segment: usize,
    cpu: &[u64],
    used: usize,
    words_per_row: usize,
) {
    let used_bits = align_used_bits(air_id, words_per_row);
    if gpu_fill_mode() == GpuFillMode::ArenaCheck {
        let mut gpu = vec![0u64; cpu.len()];
        match zisk_sm_mem_planner::gpu_align_witness_fill(air_id, segment, &mut gpu) {
            Ok(gpu_used) => {
                let (count, first) = compare_rows_masked(cpu, &gpu, words_per_row, used_bits);
                tracing::info!(
                    "MemAlign air {air_id}[{segment}] arena CHECK: {} words differ{} | used rows {}",
                    count,
                    first
                        .map(|(row, w, c, g)| format!(", first row {row} word {w}: cpu {c:#x} gpu {g:#x}"))
                        .unwrap_or_default(),
                    if gpu_used == used { "match".to_string() } else { format!("DIFFER (cpu {used}, gpu {gpu_used})") }
                );
            }
            Err(e) => {
                tracing::warn!("MemAlign air {air_id}[{segment}] arena CHECK unavailable: {e}")
            }
        }
    }
    align_dump(air_id, segment, cpu, used, words_per_row);
}

/// The hash of a MemAlign instance's rows and used-row count, under the name `align<air_id>`.
pub(crate) fn align_dump(
    air_id: usize,
    segment: usize,
    rows: &[u64],
    used: usize,
    words_per_row: usize,
) {
    crate::mem_trace_hash::dump_scalars(
        &format!("align{air_id}"),
        segment,
        rows,
        &[used as u64],
        words_per_row,
        align_used_bits(air_id, words_per_row),
    );
}

fn align_used_bits(air_id: usize, words_per_row: usize) -> usize {
    packed_used_bits(zisk_pil::ZISK_AIRGROUP_ID, air_id).unwrap_or(words_per_row * 64)
}
