//! The RAM witness the GPU planner can produce from the accesses it retained.
//!
//! The runner registers its planner here once the plan is closed; the `Mem` instances of the same
//! block then ask for their rows. The planner object lives in the runner's preloaded state, so it
//! outlives the witness phase; the registration is cleared when the next block starts.

use std::sync::{Mutex, MutexGuard};

use crate::gpu_bindings::{AlignChunkEntry, AlignPlanDesc};
pub use crate::gpu_bindings::{RamFillPrepared, RamFillResult};

/// A registered planner. The raw handle is only ever used under [`GPU_RAM_WITNESS`]'s lock, which
/// serialises every device call, and only while the runner keeps the planner alive.
struct Registered {
    inner: *mut crate::gpu_bindings::CountAndPlanHandle,
}
// SAFETY: the pointer is dereferenced only through the C ABI, under the registry's mutex, so no
// two threads touch the planner at once; the planner is heap-allocated on the C++ side and is
// destroyed only by its Rust owner, after the registration was cleared.
unsafe impl Send for Registered {}

static GPU_RAM_WITNESS: Mutex<Option<Registered>> = Mutex::new(None);

fn registry() -> MutexGuard<'static, Option<Registered>> {
    GPU_RAM_WITNESS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Packed column widths of an air, when it is packed.
fn packed_widths(airgroup_id: usize, air_id: usize) -> Option<Vec<u32>> {
    let info = zisk_pil::PACKED_INFO
        .iter()
        .find(|(ag, a, _)| *ag == airgroup_id && *a == air_id)
        .map(|(_, _, c)| c)?;
    if !info.is_packed {
        return None;
    }
    Some(info.unpack_info.iter().map(|&w| w as u32).collect())
}

/// Packed column widths, words per row and lanes of the `Mem` air.
fn mem_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{MemTrace, MemTraceRowPacked};
    let widths = packed_widths(MemTrace::<()>::AIRGROUP_ID, MemTrace::<()>::AIR_ID)?;
    let words = MemTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::mem_lanes_x_row() as u32;
    Some((widths, words, lanes))
}

/// Packed column widths, words per row and lanes of the `RomData` air.
fn rom_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{RomDataTrace, RomDataTraceRowPacked};
    let widths = packed_widths(RomDataTrace::<()>::AIRGROUP_ID, RomDataTrace::<()>::AIR_ID)?;
    let words = RomDataTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::rom_data_lanes_x_row() as u32;
    Some((widths, words, lanes))
}

/// Packed column widths, words per row and lanes of the `InputData` air.
fn input_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{InputDataTrace, InputDataTraceRowPacked};
    let widths = packed_widths(InputDataTrace::<()>::AIRGROUP_ID, InputDataTrace::<()>::AIR_ID)?;
    let words = InputDataTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::input_data_lanes_x_row() as u32;
    Some((widths, words, lanes))
}

/// The MemAlign airs by row layout: 0 full (MemAlign and MemAlignLarge), 1 byte (MemAlignByte and
/// MemAlignByteLarge), 2 read byte (MemAlignReadByte and its Large), 3 write byte.
fn align_air_kind(air_id: usize) -> Option<u32> {
    use zisk_pil::*;
    if air_id == MemAlignTrace::<()>::AIR_ID || air_id == MemAlignLargeTrace::<()>::AIR_ID {
        Some(0)
    } else if air_id == MemAlignByteTrace::<()>::AIR_ID
        || air_id == MemAlignByteLargeTrace::<()>::AIR_ID
    {
        Some(1)
    } else if air_id == MemAlignReadByteTrace::<()>::AIR_ID
        || air_id == MemAlignReadByteLargeTrace::<()>::AIR_ID
    {
        Some(2)
    } else if air_id == MemAlignWriteByteTrace::<()>::AIR_ID {
        Some(3)
    } else {
        None
    }
}

/// Rows of a MemAlign air.
fn align_air_rows(air_id: usize) -> Option<usize> {
    use zisk_pil::*;
    [
        (MemAlignTrace::<()>::AIR_ID, MemAlignTrace::<()>::NUM_ROWS),
        (MemAlignLargeTrace::<()>::AIR_ID, MemAlignLargeTrace::<()>::NUM_ROWS),
        (MemAlignByteTrace::<()>::AIR_ID, MemAlignByteTrace::<()>::NUM_ROWS),
        (MemAlignByteLargeTrace::<()>::AIR_ID, MemAlignByteLargeTrace::<()>::NUM_ROWS),
        (MemAlignReadByteTrace::<()>::AIR_ID, MemAlignReadByteTrace::<()>::NUM_ROWS),
        (MemAlignReadByteLargeTrace::<()>::AIR_ID, MemAlignReadByteLargeTrace::<()>::NUM_ROWS),
        (MemAlignWriteByteTrace::<()>::AIR_ID, MemAlignWriteByteTrace::<()>::NUM_ROWS),
    ]
    .into_iter()
    .find(|(a, _)| *a == air_id)
    .map(|(_, n)| n)
}

/// Packed column widths and words per row of the representative air of each MemAlign layout.
fn align_layout(kind: u32) -> Option<(Vec<u32>, u32)> {
    use proofman_fields::Goldilocks;
    use zisk_pil::*;
    let (ag, air, words) = match kind {
        0 => (
            MemAlignTrace::<()>::AIRGROUP_ID,
            MemAlignTrace::<()>::AIR_ID,
            MemAlignTraceRowPacked::<Goldilocks>::PACKED_WORDS,
        ),
        1 => (
            MemAlignByteTrace::<()>::AIRGROUP_ID,
            MemAlignByteTrace::<()>::AIR_ID,
            MemAlignByteTraceRowPacked::<Goldilocks>::PACKED_WORDS,
        ),
        2 => (
            MemAlignReadByteTrace::<()>::AIRGROUP_ID,
            MemAlignReadByteTrace::<()>::AIR_ID,
            MemAlignReadByteTraceRowPacked::<Goldilocks>::PACKED_WORDS,
        ),
        3 => (
            MemAlignWriteByteTrace::<()>::AIRGROUP_ID,
            MemAlignWriteByteTrace::<()>::AIR_ID,
            MemAlignWriteByteTraceRowPacked::<Goldilocks>::PACKED_WORDS,
        ),
        _ => return None,
    };
    Some((packed_widths(ag, air)?, words as u32))
}

/// Makes `planner` the block's RAM witness source. Tells it the chunk size and the `Mem` layout.
pub fn register_gpu_ram_witness(planner: &crate::GpuCountAndPlan, chunk_size: u64) {
    let inner = planner.raw_handle();
    let Some((widths, words, lanes)) = mem_layout() else {
        tracing::warn!("[gpu] Mem air is not packed; RAM witness on the device unavailable");
        return;
    };
    // SAFETY: `inner` is the live planner the caller holds; the slices outlive the calls.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_set_chunk_size_bits(inner, chunk_size.trailing_zeros());
        crate::gpu_bindings::count_and_plan_set_mem_layout(
            inner,
            widths.as_ptr(),
            widths.len() as u32,
            words,
            lanes,
        )
    };
    if !ok {
        tracing::warn!(
            "[gpu] Mem layout rejected by the planner; RAM witness on the device unavailable"
        );
        return;
    }
    // SAFETY: as above.
    let rom_ok = match rom_layout() {
        Some((widths, words, lanes)) => unsafe {
            crate::gpu_bindings::count_and_plan_set_rom_layout(
                inner,
                widths.as_ptr(),
                widths.len() as u32,
                words,
                lanes,
            )
        },
        None => false,
    };
    if !rom_ok {
        tracing::warn!(
            "[gpu] RomData layout rejected by the planner; its witness stays on the CPU"
        );
    }
    // SAFETY: as above.
    let input_ok = match input_layout() {
        Some((widths, words, lanes)) => unsafe {
            crate::gpu_bindings::count_and_plan_set_input_layout(
                inner,
                widths.as_ptr(),
                widths.len() as u32,
                words,
                lanes,
            )
        },
        None => false,
    };
    if !input_ok {
        tracing::warn!(
            "[gpu] InputData layout rejected by the planner; its witness stays on the CPU"
        );
    }
    for kind in 0..4u32 {
        // SAFETY: as above.
        let ok = match align_layout(kind) {
            Some((widths, words)) => unsafe {
                crate::gpu_bindings::count_and_plan_set_align_layout(
                    inner,
                    kind,
                    widths.as_ptr(),
                    widths.len() as u32,
                    words,
                )
            },
            None => false,
        };
        if !ok {
            tracing::warn!(
                "[gpu] MemAlign layout {kind} rejected by the planner; its witness stays on the CPU"
            );
        }
    }
    *registry() = Some(Registered { inner });
}

/// Builds the RomData instances `insts` (segment ids) into pinned host memory, after the RAM fill
/// and while the planner's arena is still borrowed.
pub fn gpu_rom_witness_fill_all(insts: &[u32]) -> Result<RamFillPrepared, String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let n_rows = zisk_pil::RomDataTrace::<()>::NUM_ROWS as u32;
    let mut prepared = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; `insts` outlives the call; `prepared` is a valid
    // out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_fill_all_rom_instances(
            r.inner,
            n_rows,
            insts.as_ptr(),
            insts.len() as u32,
            &mut prepared,
        )
    };
    if !ok {
        return Err(format!("fill_all_rom_instances failed with status {}", prepared.status));
    }
    Ok(prepared)
}

/// Copies RomData instance `inst`'s rows, built by [`gpu_rom_witness_fill_all`], into `out_rows`.
pub fn gpu_rom_witness_fill(
    inst: u32,
    out_rows: &mut [u64],
    n_rows: u32,
) -> Result<RamFillResult, String> {
    let words = rom_layout().ok_or("RomData air is not packed")?.1;
    // SAFETY: see `instance_rows`.
    let (src, res) = unsafe {
        instance_rows(
            inst,
            out_rows.len(),
            n_rows,
            words,
            crate::gpu_bindings::count_and_plan_rom_instance_rows,
        )?
    };
    copy_rows(out_rows, src);
    Ok(res)
}

/// Builds the InputData instances `insts` (segment ids) into pinned host memory, after the RomData
/// fill. `image` is the input region as the guest sees it, from its first byte.
pub fn gpu_input_witness_fill_all(image: &[u8], insts: &[u32]) -> Result<RamFillPrepared, String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let n_rows = zisk_pil::InputDataTrace::<()>::NUM_ROWS as u32;
    let mut prepared = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; `image` outlives the call, which copies it;
    // `prepared` is a valid out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_fill_all_input_instances(
            r.inner,
            n_rows,
            image.as_ptr(),
            image.len(),
            insts.as_ptr(),
            insts.len() as u32,
            &mut prepared,
        )
    };
    if !ok {
        return Err(format!("fill_all_input_instances failed with status {}", prepared.status));
    }
    Ok(prepared)
}

/// Copies InputData instance `inst`'s rows, built by [`gpu_input_witness_fill_all`], into `out_rows`.
pub fn gpu_input_witness_fill(
    inst: u32,
    out_rows: &mut [u64],
    n_rows: u32,
) -> Result<RamFillResult, String> {
    let words = input_layout().ok_or("InputData air is not packed")?.1;
    // SAFETY: see `instance_rows`.
    let (src, res) = unsafe {
        instance_rows(
            inst,
            out_rows.len(),
            n_rows,
            words,
            crate::gpu_bindings::count_and_plan_input_instance_rows,
        )?
    };
    copy_rows(out_rows, src);
    Ok(res)
}

/// Builds the MemAlign instances `plans` into pinned host memory, after the three memory fills.
/// Each plan carries its per-chunk checkpoints, which become the device's windows.
pub fn gpu_align_witness_fill_all(plans: &[&zisk_common::Plan]) -> Result<RamFillPrepared, String> {
    use std::collections::HashMap;
    use zisk_common::ChunkId;
    use zisk_sm_mem_common::MemAlignCheckPoint;
    let mut descs = Vec::with_capacity(plans.len());
    let mut entries: Vec<AlignChunkEntry> = Vec::new();
    for plan in plans {
        let Some(air_kind) = align_air_kind(plan.air_id) else {
            return Err(format!("plan of air {} is not a MemAlign air", plan.air_id));
        };
        let n_rows = align_air_rows(plan.air_id).ok_or("MemAlign air without rows")?;
        let segment = plan.segment_id.ok_or("MemAlign plan without segment")?;
        let checkpoints = plan
            .meta
            .as_ref()
            .and_then(|m| m.downcast_ref::<HashMap<ChunkId, MemAlignCheckPoint>>())
            .ok_or("MemAlign plan without checkpoints")?;
        let mut chunks: Vec<&MemAlignCheckPoint> = checkpoints.values().collect();
        chunks.sort_by_key(|c| c.chunk_id);
        let entry_from = entries.len() as u32;
        for c in chunks {
            let w = |k: &zisk_common::CollectCounter| (k.initial_skip, k.collect_count);
            let (s5, n5) = w(&c.full_5);
            let (s3, n3) = w(&c.full_3);
            let (s2, n2) = w(&c.full_2);
            let (sr, nr) = w(&c.read_byte);
            let (sw, nw) = w(&c.write_byte);
            entries.push(AlignChunkEntry {
                chunk: c.chunk_id.0 as u32,
                skip: [s5, s3, s2, sr, sw],
                count: [n5, n3, n2, nr, nw],
            });
        }
        descs.push(AlignPlanDesc {
            air_kind,
            air_id: plan.air_id as u32,
            segment: usize::from(segment) as u32,
            n_rows: n_rows as u32,
            entry_from,
            entry_n: entries.len() as u32 - entry_from,
        });
    }
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let mut prepared = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; the tables outlive the call; `prepared` is a
    // valid out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_fill_all_align_instances(
            r.inner,
            descs.as_ptr(),
            descs.len() as u32,
            entries.as_ptr(),
            entries.len() as u32,
            &mut prepared,
        )
    };
    if !ok {
        return Err(format!("fill_all_align_instances failed with status {}", prepared.status));
    }
    Ok(prepared)
}

/// Copies the rows of MemAlign instance (`air_id`, `segment`), built by
/// [`gpu_align_witness_fill_all`], into `out_rows`; returns the rows the accesses use.
pub fn gpu_align_witness_fill(
    air_id: usize,
    segment: usize,
    out_rows: &mut [u64],
) -> Result<usize, String> {
    let kind = align_air_kind(air_id).ok_or("not a MemAlign air")?;
    let words = align_layout(kind).ok_or("MemAlign air is not packed")?.1 as usize;
    let n_rows = align_air_rows(air_id).ok_or("MemAlign air without rows")?;
    if out_rows.len() != n_rows * words {
        return Err(format!(
            "out_rows holds {} words, {n_rows} rows need {}",
            out_rows.len(),
            n_rows * words
        ));
    }
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let mut res = RamFillResult::default();
    // SAFETY: registered handle, under the lock; `res` is a valid out-parameter.
    let p = unsafe {
        crate::gpu_bindings::count_and_plan_align_instance_rows(
            r.inner,
            air_id as u32,
            segment as u32,
            &mut res,
        )
    };
    if p.is_null() {
        return Err(format!(
            "MemAlign air {air_id} segment {segment} has no rows: the device fill did not run or failed"
        ));
    }
    // SAFETY: the pinned rows buffer holds `n_rows * words` words for this instance and stays
    // valid until the planner's next block (reset), which cannot start before this witness phase
    // ends.
    let src = unsafe { std::slice::from_raw_parts(p, n_rows * words) };
    copy_rows(out_rows, src);
    Ok(res.n_lanes as usize)
}

/// The pinned rows of instance `inst` through `rows_fn`, checked against the caller's row count.
///
/// # Safety
/// `rows_fn` must be one of the planner's `*_instance_rows` entry points.
unsafe fn instance_rows(
    inst: u32,
    out_len: usize,
    n_rows: u32,
    words: u32,
    rows_fn: unsafe extern "C" fn(
        *mut crate::gpu_bindings::CountAndPlanHandle,
        u32,
        *mut RamFillResult,
    ) -> *const u64,
) -> Result<(&'static [u64], RamFillResult), String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let len = n_rows as usize * words as usize;
    if out_len != len {
        return Err(format!("out_rows holds {out_len} words, {n_rows} rows need {len}"));
    }
    let mut res = RamFillResult::default();
    // SAFETY: registered handle, under the lock; `res` is a valid out-parameter.
    let p = unsafe { rows_fn(r.inner, inst, &mut res) };
    if p.is_null() {
        return Err(format!("instance {inst} has no rows: the device fill did not run or failed"));
    }
    // SAFETY: the pinned rows buffer holds `len` words for this instance and stays valid until the
    // planner's next block (reset), which cannot start before this witness phase ends.
    Ok((unsafe { std::slice::from_raw_parts(p, len) }, res))
}

/// Copies the rows out with several threads: the copy is the only remaining work, and it is large.
fn copy_rows(out_rows: &mut [u64], src: &[u64]) {
    let threads = 8usize;
    let per = src.len().div_ceil(threads);
    std::thread::scope(|sc| {
        for (d, s) in out_rows.chunks_mut(per).zip(src.chunks(per)) {
            sc.spawn(move || d.copy_from_slice(s));
        }
    });
}

/// Forgets the registered planner (the next block is starting).
pub fn clear_gpu_ram_witness() {
    *registry() = None;
}

/// Whether a planner is registered and retained every RAM access of the block.
pub fn gpu_ram_witness_available() -> bool {
    let reg = registry();
    match reg.as_ref() {
        // SAFETY: registered handle, under the lock.
        Some(r) => unsafe { crate::gpu_bindings::count_and_plan_ram_retention_ok(r.inner) },
        None => false,
    }
}

/// Builds the RAM instances `insts` (segment ids) into pinned host memory. Must run while the
/// planner's arena is still borrowed; the witness phase then only copies.
pub fn gpu_ram_witness_fill_all(insts: &[u32]) -> Result<RamFillPrepared, String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the RAM witness")?;
    let n_rows = zisk_pil::MemTrace::<()>::NUM_ROWS as u32;
    let mut prepared = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; `insts` outlives the call; `prepared` is a valid
    // out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_fill_all_ram_instances(
            r.inner,
            n_rows,
            insts.as_ptr(),
            insts.len() as u32,
            &mut prepared,
        )
    };
    if !ok {
        return Err(format!("fill_all_ram_instances failed with status {}", prepared.status));
    }
    Ok(prepared)
}

/// Sorts the retained accesses into lanes and resolves the read values. Idempotent per block.
pub fn gpu_ram_witness_prepare() -> Result<RamFillPrepared, String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the RAM witness")?;
    let mut out = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; `out` is a valid out-parameter.
    let ok = unsafe { crate::gpu_bindings::count_and_plan_prepare_ram_fill(r.inner, &mut out) };
    if !ok {
        return Err(format!("prepare_ram_fill failed with status {}", out.status));
    }
    Ok(out)
}

/// Copies RAM instance `inst`'s rows, built by [`gpu_ram_witness_fill_all`], into `out_rows`
/// (`n_rows` rows of the `Mem` packed layout).
pub fn gpu_ram_witness_fill(
    inst: u32,
    out_rows: &mut [u64],
    n_rows: u32,
) -> Result<RamFillResult, String> {
    let words = mem_layout().ok_or("Mem air is not packed")?.1;
    // SAFETY: see `instance_rows`.
    let (src, res) = unsafe {
        instance_rows(
            inst,
            out_rows.len(),
            n_rows,
            words,
            crate::gpu_bindings::count_and_plan_ram_instance_rows,
        )?
    };
    copy_rows(out_rows, src);
    Ok(res)
}
