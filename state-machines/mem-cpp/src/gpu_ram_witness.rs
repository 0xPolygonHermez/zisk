//! The RAM witness the GPU planner can produce from the accesses it retained.
//!
//! The runner registers its planner here once the plan is closed; the `Mem` instances of the same
//! block then ask for their rows. The planner object lives in the runner's preloaded state, so it
//! outlives the witness phase; the registration is cleared when the next block starts.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::gpu_bindings::{AlignChunkEntry, AlignPlanDesc, StagedRows};
pub use crate::gpu_bindings::{MemSlotOp, RamFillPrepared, RamFillResult};

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
    if ASYNC_PREP.load(Ordering::Acquire) {
        return gpu_mem_witness_rows_into(1, 0, inst, n_rows, out_rows);
    }
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
    if ASYNC_PREP.load(Ordering::Acquire) {
        return gpu_mem_witness_rows_into(2, 0, inst, n_rows, out_rows);
    }
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

/// The device's view of MemAlign plans: one descriptor per plan, its chunks' per-kind windows in
/// `entries` (the checkpoints the plan carries).
fn align_tables(
    plans: &[&zisk_common::Plan],
) -> Result<(Vec<AlignPlanDesc>, Vec<AlignChunkEntry>), String> {
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
    Ok((descs, entries))
}

/// Builds the MemAlign instances `plans` into pinned host memory, after the three memory fills.
/// Each plan carries its per-chunk checkpoints, which become the device's windows.
pub fn gpu_align_witness_fill_all(plans: &[&zisk_common::Plan]) -> Result<RamFillPrepared, String> {
    let (descs, entries) = align_tables(plans)?;
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
    if ASYNC_PREP.load(Ordering::Acquire) {
        return gpu_mem_witness_rows_into(
            3,
            air_id as u32,
            segment as u32,
            n_rows as u32,
            out_rows,
        )
        .map(|r| r.n_lanes as usize);
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
    ASYNC_PREP.store(false, Ordering::Release);
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
    if ASYNC_PREP.load(Ordering::Acquire) {
        return Ok(*PREP_INFO.lock().unwrap_or_else(|e| e.into_inner()));
    }
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
    if ASYNC_PREP.load(Ordering::Acquire) {
        return gpu_mem_witness_rows_into(0, 0, inst, n_rows, out_rows);
    }
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

// ─── Memory instances filled into the prover's slots (ZISK_MEM_GPU_FILL=slot) ─────────────────

/// What runs when the last owned memory instance has been filled into its slot: the arena goes
/// back to the prover, and the memory airs leave the device for the rest of the block.
struct SlotRelease {
    pending: usize,
    release: Option<Box<dyn FnOnce() + Send>>,
}

static SLOT_RELEASE: Mutex<SlotRelease> = Mutex::new(SlotRelease { pending: 0, release: None });

/// The block's slot preparation, running on its own thread; joined by the first slot fill or
/// scalar read of the block, or by the arena's release.
struct SlotPrepare {
    handle: Option<std::thread::JoinHandle<Result<RamFillPrepared, String>>>,
    done: Option<Result<(), String>>,
}
static SLOT_PREPARE: Mutex<SlotPrepare> = Mutex::new(SlotPrepare { handle: None, done: None });
/// The block's memory witness is being prepared on the device (`arena` or `slot`): the rows come
/// from the staged images, not from the synchronous fills.
static ASYNC_PREP: AtomicBool = AtomicBool::new(false);
/// What the preparation reported, once done (zeros before).
static PREP_INFO: Mutex<RamFillPrepared> = Mutex::new(RamFillPrepared {
    status: 0,
    n_instances: 0,
    n_accesses: 0,
    n_lanes: 0,
    ms_sort: 0.0,
    ms_lanes: 0.0,
    ms_values: 0.0,
    ms_total: 0.0,
});

/// The rows of one memory instance (`family` 0 Mem, 1 RomData, 2 InputData, 3 MemAlign by
/// `air_id`) into `out_rows` (`n_rows` packed rows), from the image the preparation staged, or
/// built from the retained accesses when it staged none; waits for the preparation only as far as
/// this instance. Counts toward the arena's release.
pub fn gpu_mem_witness_rows_into(
    family: u32,
    air_id: u32,
    segment: u32,
    n_rows: u32,
    out_rows: &mut [u64],
) -> Result<RamFillResult, String> {
    let inner = {
        let reg = registry();
        reg.as_ref().ok_or("no GPU planner registered for the memory witness")?.inner as usize
    };
    let inner = inner as *mut crate::gpu_bindings::CountAndPlanHandle;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    let staged = loop {
        let mut s = StagedRows::default();
        // SAFETY: the planner outlives the block; the call only reads the planner's staging table
        // under its own lock; `s` is a valid out-parameter.
        let rc = unsafe {
            crate::gpu_bindings::count_and_plan_staged_wait(
                inner, family, air_id, segment, 200, &mut s,
            )
        };
        match rc {
            1 => {
                if s.n_rows != n_rows || s.words as usize != out_rows.len() {
                    return Err(format!(
                        "family {family} air {air_id} segment {segment}: staged {} rows x {} words, the trace holds {n_rows} rows, {} words",
                        s.n_rows, s.words, out_rows.len()
                    ));
                }
                // SAFETY: `out_rows` holds exactly `words` words; the copy runs on the planner's own
                // stream and returns complete.
                let ok = unsafe {
                    crate::gpu_bindings::count_and_plan_copy_staged(
                        inner,
                        family,
                        air_id,
                        segment,
                        out_rows.as_mut_ptr(),
                        s.words,
                    )
                };
                break if ok { Some(s.res) } else { None };
            }
            0 => break None,
            _ if std::time::Instant::now() < deadline => continue,
            _ => return Err("timed out waiting for the memory witness preparation".into()),
        }
    };
    let res = match staged {
        Some(res) => res,
        None => {
            // Not staged (no room, or the image was dropped): the preparation's outcome first, then
            // the full fill into host memory, serialised with every other planner call.
            slot_prepare_join()?;
            let reg = registry();
            let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
            let mut res = RamFillResult::default();
            // SAFETY: registered handle, under the lock; `out_rows` holds `n_rows` rows of the
            // instance's layout (checked by the fill); `res` is a valid out-parameter.
            let ok = unsafe {
                crate::gpu_bindings::count_and_plan_fill_host(
                    r.inner,
                    family,
                    air_id,
                    segment,
                    n_rows,
                    out_rows.as_mut_ptr(),
                    &mut res,
                )
            };
            if !ok {
                return Err(format!(
                    "family {family} air {air_id} segment {segment}: the device fill failed with status {}",
                    res.status
                ));
            }
            res
        }
    };
    if let Some(release) = slot_fill_done() {
        slot_quiesce();
        ASYNC_PREP.store(false, Ordering::Release);
        release();
    }
    Ok(res)
}

/// Starts the block's preparation for the slot fills (every instance resolved, scalars and
/// MemAlign old words, the input image on the device, the owned MemAlign plans known to the
/// planner) on its own thread, off the executor's path: it runs beside the registration of the
/// secondaries and the first commits, and the first slot fill waits for it. The host-side checks
/// stay synchronous, so a block the device cannot serve still falls back before any instance is
/// built: the retained accesses must be complete and the MemAlign tables well formed.
///
/// With `host_rows`, the owned instances' rows are also copied to pinned host memory behind the
/// preparation, and [`gpu_mem_witness_host_rows_mask`] tells which families have every owned
/// instance there: once the arena is released, the proofs take the rows through the
/// `gpu_*_witness_fill` copies instead of rebuilding them on the CPU.
pub fn gpu_slot_witness_prepare_async(
    image: Vec<u8>,
    ram: Vec<u32>,
    rom: Vec<u32>,
    input: Vec<u32>,
    align_plans: &[&zisk_common::Plan],
    host_rows: bool,
) -> Result<(), String> {
    let (descs, entries) = align_tables(align_plans)?;
    let inner = {
        let reg = registry();
        let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
        // SAFETY: registered handle, under the lock.
        if !unsafe { crate::gpu_bindings::count_and_plan_ram_retention_ok(r.inner) } {
            return Err("the retained accesses are incomplete for this block".into());
        }
        r.inner as usize
    };
    ASYNC_PREP.store(true, Ordering::Release);
    *PREP_INFO.lock().unwrap_or_else(|e| e.into_inner()) = RamFillPrepared::default();
    let handle = std::thread::Builder::new()
        .name("mem-slot-prepare".into())
        .spawn(move || {
            let inner = inner as *mut crate::gpu_bindings::CountAndPlanHandle;
            let mut prepared = RamFillPrepared::default();
            // SAFETY: the planner outlives the block (the next block's runner clears the
            // registration only after the release joined this thread). The registry lock is not
            // held: the only other planner calls while this runs are `staged_wait` and
            // `copy_staged`, which the planner serialises against the preparation with its own
            // lock and run on their own streams; the fills themselves wait for the join. The
            // tables and the image outlive the call, which copies what it keeps; `prepared` is a
            // valid out-parameter.
            let ok = unsafe {
                crate::gpu_bindings::count_and_plan_prepare_slot_fills(
                    inner,
                    image.as_ptr(),
                    image.len(),
                    descs.as_ptr(),
                    descs.len() as u32,
                    entries.as_ptr(),
                    entries.len() as u32,
                    ram.as_ptr(),
                    ram.len() as u32,
                    rom.as_ptr(),
                    rom.len() as u32,
                    input.as_ptr(),
                    input.len() as u32,
                    host_rows,
                    &mut prepared,
                )
            };
            if !ok {
                return Err(format!("prepare_slot_fills failed with status {}", prepared.status));
            }
            *PREP_INFO.lock().unwrap_or_else(|e| e.into_inner()) = prepared;
            tracing::info!(
                "[gpu] memory witness prepared on the device: {} accesses resolved, {} instances",
                prepared.n_accesses,
                prepared.n_instances
            );
            Ok(prepared)
        })
        .map_err(|e| format!("cannot start the slot preparation thread: {e}"))?;
    let mut g = SLOT_PREPARE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(stale) = g.handle.take() {
        let _ = stale.join();
    }
    g.done = None;
    g.handle = Some(handle);
    Ok(())
}

/// Waits for the block's slot preparation; its failure is every fill's failure. The lock is
/// held through the join, so a second waiter blocks until the result is known.
fn slot_prepare_join() -> Result<(), String> {
    let mut g = SLOT_PREPARE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = g.handle.take() {
        g.done = Some(match h.join() {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("the slot preparation thread panicked".into()),
        });
    }
    g.done.clone().unwrap_or(Ok(()))
}

/// The scalars of a resolved instance (`family` 0 Mem, 1 RomData, 2 InputData), for its air
/// values; valid after the slot preparation, with or without rows.
pub fn gpu_mem_witness_scalars(family: u32, inst: u32) -> Result<RamFillResult, String> {
    slot_prepare_join()?;
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let mut res = RamFillResult::default();
    // SAFETY: registered handle, under the lock; `res` is a valid out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_instance_scalars(r.inner, family, inst, &mut res)
    };
    if !ok {
        return Err(format!("instance {inst} of family {family} was not resolved on the device"));
    }
    Ok(res)
}

/// `MEM_ROWS_*` bits of the families whose owned instances all have their rows in pinned host
/// memory (zero before the preparation ends, or without host rows).
pub fn gpu_mem_witness_host_rows_mask() -> u32 {
    if slot_prepare_join().is_err() {
        return 0;
    }
    let reg = registry();
    match reg.as_ref() {
        // SAFETY: registered handle, under the lock.
        Some(r) => unsafe { crate::gpu_bindings::count_and_plan_host_rows_mask(r.inner) },
        None => 0,
    }
}

/// Arms the release that follows the last of `n_pending` slot fills.
pub fn gpu_slot_witness_arm(n_pending: usize, release: Box<dyn FnOnce() + Send>) {
    let mut g = SLOT_RELEASE.lock().unwrap_or_else(|e| e.into_inner());
    g.pending = n_pending;
    g.release = Some(release);
}

/// Runs the armed release now, if it has not run: the end of the proof, or a block whose memory
/// instances were not all committed.
pub fn gpu_slot_witness_release_now() {
    // The preparation may still be using the arena: never release it underneath.
    let _ = slot_prepare_join();
    let release = {
        let mut g = SLOT_RELEASE.lock().unwrap_or_else(|e| e.into_inner());
        g.pending = 0;
        g.release.take()
    };
    ASYNC_PREP.store(false, Ordering::Release);
    if let Some(release) = release {
        slot_quiesce();
        release();
    }
}

/// The release to run after this slot fill, when it was the last pending one.
fn slot_fill_done() -> Option<Box<dyn FnOnce() + Send>> {
    let mut g = SLOT_RELEASE.lock().unwrap_or_else(|e| e.into_inner());
    if g.pending == 0 {
        return None;
    }
    g.pending -= 1;
    if g.pending == 0 {
        g.release.take()
    } else {
        None
    }
}

/// Waits for the staged images' copies still in flight on the prover's streams: the arena they
/// read from is about to go back.
fn slot_quiesce() {
    let reg = registry();
    if let Some(r) = reg.as_ref() {
        // SAFETY: registered handle, under the lock.
        unsafe { crate::gpu_bindings::count_and_plan_slot_quiesce(r.inner) };
    }
}

/// The prover's GPU-witness kernel for the memory airs: copies the instance named by the staged
/// `MemSlotOp` into `d_dst` (the commit slot) from the image the preparation built, or builds it
/// there from the retained accesses when no image was staged, while the arena is still borrowed;
/// after the last owned instance the armed release hands the arena back. The slot must be on the
/// planner's GPU: the rows are produced there, never copied across devices.
///
/// # Safety
/// Called by the prover with `d_ops` a device buffer holding `num_ops` staged ops, `d_dst` the
/// slot's packed rows, `device_id` the slot's GPU and `stream` the commit stream that uploaded
/// them; the planner waits for that stream and serialises the fills under its lock. A staged
/// image is copied on `stream`; a built one is complete before the call returns.
pub unsafe extern "C" fn zisk_mem_witness_slot_kernel(
    d_ops: *const core::ffi::c_void,
    num_ops: u64,
    d_dst: *mut u64,
    device_id: i32,
    stream: *mut core::ffi::c_void,
) -> i32 {
    if let Err(e) = slot_prepare_join() {
        tracing::error!("[gpu] slot fill: the block's preparation failed: {e}");
        return -4;
    }
    let release = {
        let reg = registry();
        let Some(r) = reg.as_ref() else {
            tracing::error!("[gpu] slot fill: no GPU planner registered");
            return -1;
        };
        // SAFETY: registered handle, under the lock.
        let planner_device = unsafe { crate::gpu_bindings::count_and_plan_device(r.inner) };
        if device_id != planner_device {
            tracing::error!(
                "[gpu] slot fill: the slot is on GPU {device_id} but the planner holds the accesses on GPU {planner_device};                  the memory airs must commit on the planner's GPU"
            );
            return -3;
        }
        let mut res = RamFillResult::default();
        // SAFETY: registered handle, under the lock; the prover's pointers are valid for the call.
        let ok = unsafe {
            crate::gpu_bindings::count_and_plan_fill_slot(
                r.inner, d_ops, num_ops, d_dst, stream, &mut res,
            )
        };
        if !ok {
            tracing::error!("[gpu] slot fill failed with status {}", res.status);
            return -2;
        }
        let release = slot_fill_done();
        if release.is_some() {
            // SAFETY: registered handle, under the lock.
            unsafe { crate::gpu_bindings::count_and_plan_slot_quiesce(r.inner) };
            ASYNC_PREP.store(false, Ordering::Release);
        }
        release
    };
    if let Some(release) = release {
        release();
    }
    0
}
