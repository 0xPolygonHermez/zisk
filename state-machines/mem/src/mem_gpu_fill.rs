//! The `Mem` witness from the accesses the GPU planner retained, behind `ZISK_MEM_GPU_FILL`.
//!
//! `arena` proves the device rows and builds no RAM collectors. `arena-check` keeps the collectors,
//! fills both ways and compares every word and scalar; the CPU rows are what gets proved.

use proofman_fields::PrimeField64;
use zisk_common::SegmentId;
use zisk_pil::{MemTrace, MemTraceRowPacked, PACKED_INFO};
use zisk_sm_mem_common::{RAM_W_ADDR_END, RAM_W_ADDR_INIT};

use crate::mem_sm::{split_last_step, split_padding_size, MemFillOutput, MemPreviousSegment};

/// A fill's observer: the filled rows as words and the fill's scalars, before the air instance is
/// built. The arena check compares them with the device rows.
pub type OnFilled<'a, O> = Option<&'a mut dyn FnMut(&[u64], &O)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpuFillMode {
    Off,
    /// Fill from the planner's retained accesses, compared against the CPU fill.
    ArenaCheck,
    /// Fill from the planner's retained accesses, no collectors; the device rows are proved.
    Arena,
}

/// `ZISK_MEM_GPU_FILL`: unset or anything else is `Off`.
pub(crate) fn gpu_fill_mode() -> GpuFillMode {
    match std::env::var("ZISK_MEM_GPU_FILL").as_deref() {
        Ok("arena-check") => GpuFillMode::ArenaCheck,
        Ok("arena") => GpuFillMode::Arena,
        _ => GpuFillMode::Off,
    }
}

/// Packed column widths of the `Mem` air, in packed column order.
fn mem_col_widths() -> Vec<u32> {
    let airgroup_id = MemTrace::<()>::AIRGROUP_ID;
    let air_id = MemTrace::<()>::AIR_ID;
    let info = PACKED_INFO
        .iter()
        .find(|(ag, a, _)| *ag == airgroup_id && *a == air_id)
        .map(|(_, _, c)| c)
        .expect("Mem air has no packed info");
    assert!(info.is_packed, "Mem air is not packed");
    info.unpack_info.iter().map(|&w| w as u32).collect()
}

/// The words of packed rows, for the device to write directly.
pub(crate) fn packed_rows_as_words<F: PrimeField64>(
    rows: &mut [MemTraceRowPacked<F>],
) -> &mut [u64] {
    let words = MemTraceRowPacked::<F>::PACKED_WORDS;
    assert_eq!(
        std::mem::size_of::<MemTraceRowPacked<F>>(),
        words * 8,
        "MemTraceRowPacked is not exactly its packed words"
    );
    // SAFETY: `MemTraceRowPacked<F>` is `#[repr(C)]` with a zero-sized `PhantomData<F>` and a
    // `[u64; PACKED_WORDS]`, so (as asserted above) a row is exactly `PACKED_WORDS` u64 words with
    // u64 alignment. The slice covers precisely the rows' memory and inherits their exclusive borrow.
    unsafe { std::slice::from_raw_parts_mut(rows.as_mut_ptr() as *mut u64, rows.len() * words) }
}

/// What the planner's fill of one RAM instance hands back, in the fill's own terms.
pub(crate) struct ArenaFillReport {
    pub prepared: zisk_sm_mem_planner::RamFillPrepared,
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
    if !zisk_sm_mem_planner::gpu_ram_witness_available() {
        return Err("the GPU planner did not retain this block's RAM accesses".to_string());
    }
    // The device phase ran in the runner while the arena was borrowed; this only copies.
    let prepared = zisk_sm_mem_planner::gpu_ram_witness_prepare()?;
    let res = zisk_sm_mem_planner::gpu_ram_witness_fill(
        usize::from(segment_id) as u32,
        rows,
        n_rows as u32,
    )?;
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
    Ok(ArenaFillReport { prepared, res, previous_segment, out })
}

/// One column group's mismatches: name, differing lanes, first as (row, lane, cpu, gpu).
pub(crate) type ColumnDiff = (&'static str, usize, Option<(usize, usize, u64, u64)>);

/// Mismatches per column group between two packed row buffers: for every group, the number of
/// lanes whose column differs and the first such lane as (row, lane, cpu, gpu). Groups follow the
/// packed declaration order: addr, step, addr_changes, step_dual, sel_dual, value(2 per lane), wr,
/// previous_step, l_increment, h_increment, read_same_addr.
pub(crate) fn compare_rows_by_column(
    cpu: &[u64],
    gpu: &[u64],
    words_per_row: usize,
    lanes: usize,
) -> Vec<ColumnDiff> {
    const GROUPS: [&str; 11] = [
        "addr",
        "step",
        "addr_changes",
        "step_dual",
        "sel_dual",
        "value",
        "wr",
        "previous_step",
        "l_increment",
        "h_increment",
        "read_same_addr",
    ];
    let widths = mem_col_widths();
    let mut offsets = Vec::with_capacity(widths.len());
    let mut off = 0usize;
    for w in &widths {
        offsets.push(off);
        off += *w as usize;
    }
    let get = |row: &[u64], c: usize| -> u64 {
        let (o, w) = (offsets[c], widths[c] as usize);
        let (ws, bs) = (o / 64, o % 64);
        let mut v = row[ws] >> bs;
        if bs + w > 64 {
            v |= row[ws + 1] << (64 - bs);
        }
        if w < 64 {
            v & ((1u64 << w) - 1)
        } else {
            v
        }
    };
    let mut out: Vec<ColumnDiff> = GROUPS.iter().map(|g| (*g, 0usize, None)).collect();
    let n_rows = cpu.len() / words_per_row;
    for r in 0..n_rows {
        let a = &cpu[r * words_per_row..(r + 1) * words_per_row];
        let b = &gpu[r * words_per_row..(r + 1) * words_per_row];
        if a == b {
            continue;
        }
        let mut c = 0usize;
        for (g, entry) in out.iter_mut().enumerate() {
            let per_lane = if g == 5 { 2 } else { 1 };
            for l in 0..lanes {
                let mut differs = false;
                for k in 0..per_lane {
                    if get(a, c + k) != get(b, c + k) {
                        differs = true;
                    }
                }
                if differs {
                    entry.1 += 1;
                    if entry.2.is_none() {
                        entry.2 = Some((r, l, get(a, c), get(b, c)));
                    }
                }
                c += per_lane;
            }
        }
    }
    out
}

/// The bits a packed row of the air uses, counted from its first word; `None` when not packed.
pub(crate) fn packed_used_bits(airgroup_id: usize, air_id: usize) -> Option<usize> {
    PACKED_INFO
        .iter()
        .find(|(ag, a, _)| *ag == airgroup_id && *a == air_id)
        .filter(|(_, _, c)| c.is_packed)
        .map(|(_, _, c)| c.unpack_info.iter().map(|&w| w as usize).sum())
}

/// Like [`compare_rows`], ignoring the bits of each row's last word beyond `used_bits`: the CPU
/// fill of an air that does not zero its trace leaves whatever the buffer held there, which the
/// prover never reads.
pub(crate) fn compare_rows_masked(
    cpu: &[u64],
    gpu: &[u64],
    words_per_row: usize,
    used_bits: usize,
) -> (usize, Option<(usize, usize, u64, u64)>) {
    let tail_bits = used_bits - 64 * (words_per_row - 1);
    let tail_mask = if tail_bits >= 64 { u64::MAX } else { (1u64 << tail_bits) - 1 };
    let mut count = 0;
    let mut first = None;
    for (i, (a, b)) in cpu.iter().zip(gpu.iter()).enumerate() {
        let w = i % words_per_row;
        let (a, b) = if w == words_per_row - 1 { (a & tail_mask, b & tail_mask) } else { (*a, *b) };
        if a != b {
            count += 1;
            if first.is_none() {
                first = Some((i / words_per_row, w, a, b));
            }
        }
    }
    (count, first)
}

/// First differing word between two row buffers, as (row, word, cpu, gpu), and the count.
pub(crate) fn compare_rows(
    cpu: &[u64],
    gpu: &[u64],
    words_per_row: usize,
) -> (usize, Option<(usize, usize, u64, u64)>) {
    let mut count = 0;
    let mut first = None;
    for (i, (a, b)) in cpu.iter().zip(gpu.iter()).enumerate() {
        if a != b {
            count += 1;
            if first.is_none() {
                first = Some((i / words_per_row, i % words_per_row, *a, *b));
            }
        }
    }
    (count, first)
}
