//! The `Mem` witness from the accesses the GPU planner retained (`ZISK_MEM_GPU_FILL`).

use zisk_common::SegmentId;
use zisk_sm_mem_common::{RAM_W_ADDR_END, RAM_W_ADDR_INIT};

use crate::mem_sm::{split_last_step, split_padding_size, MemFillOutput, MemPreviousSegment};

/// A fill's observer: the filled rows as words and the fill's scalars, before the air instance is
/// built. The arena check compares them with the device rows.
pub(crate) struct ArenaFillReport {
    pub res: zisk_sm_mem_planner::RamFillResult,
    pub previous_segment: MemPreviousSegment,
    pub out: MemFillOutput,
}

/// Fills `rows` (`n_rows` packed rows as words) of RAM instance `segment_id` from the accesses the
/// GPU planner retained for this block.
pub(crate) fn arena_fill_packed_rows(
    rows: &mut [u64],
    n_rows: usize,
    segment_id: SegmentId,
) -> Result<ArenaFillReport, String> {
    let res = zisk_sm_mem_planner::gpu_ram_witness_fill(
        usize::from(segment_id) as u32,
        rows,
        n_rows as u32,
    )?;
    let (previous_segment, out) = mem_report(&res, n_rows);
    Ok(ArenaFillReport { res, previous_segment, out })
}

/// The scalars a filled Mem instance's air values are built from, out of the planner's result.
pub(crate) fn mem_report(
    res: &zisk_sm_mem_planner::RamFillResult,
    n_rows: usize,
) -> (MemPreviousSegment, MemFillOutput) {
    let previous_segment =
        MemPreviousSegment { addr: res.prev_addr_w, step: res.prev_step, value: res.prev_value };
    // The padding (`MemPadding` in mem_sm.rs) reads the last word of the region: when there is
    // any, that word is the LAST lane the air values describe, holding the last access's value
    // only if that access already sat there.
    let num_slots = n_rows * zisk_sm_mem_common::mem_lanes_x_row();
    let padding_size = (num_slots - res.n_lanes as usize) as u32;
    let (last_addr, last_value) = if padding_size > 0 {
        let value = if res.last_addr_w == RAM_W_ADDR_END { res.last_value } else { 0 };
        (RAM_W_ADDR_END, value)
    } else {
        (res.last_addr_w, res.last_value)
    };
    let distance_base = res.prev_addr_w - RAM_W_ADDR_INIT;
    let distance_end = RAM_W_ADDR_END - last_addr;
    let (padding_size_chunks, padding_size_to_max_chunks) =
        split_padding_size(padding_size, (num_slots - 1) as u32);
    let out = MemFillOutput {
        last_addr,
        last_step: res.last_step,
        last_value: [last_value as u32, (last_value >> 32) as u32],
        distance_base: [distance_base as u16, (distance_base >> 16) as u16],
        distance_end: [distance_end as u16, (distance_end >> 16) as u16],
        last_step_chunks: split_last_step(res.last_step),
        padding_size,
        padding_size_chunks,
        padding_size_to_max_chunks,
    };
    (previous_segment, out)
}

/// The rows' memory as words, for a packed row type the device writes directly.
pub(crate) fn rows_as_words_mut<R>(rows: &mut [R]) -> &mut [u64] {
    let bytes = std::mem::size_of_val(rows);
    assert_eq!(bytes % 8, 0);
    assert_eq!(std::mem::align_of::<R>() % 8, 0);
    // SAFETY: `rows` is a contiguous, 8-byte-aligned slice of `#[repr(C)]` packed rows whose fields
    // are all 8-byte words; the slice covers precisely the rows' memory and inherits their
    // exclusive borrow.
    unsafe { std::slice::from_raw_parts_mut(rows.as_mut_ptr() as *mut u64, bytes / 8) }
}

#[cfg(feature = "mem-witness-test")]
pub(crate) fn rows_as_words<R>(rows: &[R]) -> &[u64] {
    let bytes = std::mem::size_of_val(rows);
    assert_eq!(bytes % 8, 0);
    // SAFETY: `rows` is a contiguous, initialised, 8-byte-aligned slice of `#[repr(C)]` rows whose
    // fields are all 8-byte words; reinterpreting its bytes as `u64` reads no padding.
    unsafe { std::slice::from_raw_parts(rows.as_ptr() as *const u64, bytes / 8) }
}

/// The bits a packed row of the air uses, counted from its first word; `None` when not packed.
#[cfg(feature = "mem-witness-test")]
pub(crate) fn packed_used_bits(airgroup_id: usize, air_id: usize) -> Option<usize> {
    zisk_pil::PACKED_INFO
        .iter()
        .find(|(ag, a, _)| *ag == airgroup_id && *a == air_id)
        .filter(|(_, _, c)| c.is_packed)
        .map(|(_, _, c)| c.unpack_info.iter().map(|&w| w as usize).sum())
}
