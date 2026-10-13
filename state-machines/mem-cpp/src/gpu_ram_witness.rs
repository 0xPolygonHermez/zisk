//! The memory witness the GPU planner builds from the accesses it retained.
//!
//! The runner registers its planner here once the plan is closed and starts the block's
//! preparation on its own thread; the memory instances of the block then take their rows
//! (`gpu_*_witness_fill`), in the witness phase from the images the preparation staged on the
//! device, in the proof phase from the pinned host copy. The planner object lives in the runner's
//! preloaded state, so it outlives the witness phase; the registration is cleared when the next
//! block starts.
//!
//! Every failure is an `Err` the caller turns into the CPU witness for that instance; no device
//! failure is ever folded into rows.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::gpu_bindings::{AlignChunkEntry, AlignPlanDesc, StagedRows};
use crate::mem_witness_types::{
    RamFillPrepared, RamFillResult, MEM_FAMILY_ALIGN, MEM_FAMILY_INPUT, MEM_FAMILY_RAM,
    MEM_FAMILY_ROM,
};

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

/// Calls inside [`rows_into`] or the slot kernel: the registration waits for them to end before
/// it is cleared (`clear_gpu_ram_witness`).
static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

fn registry() -> MutexGuard<'static, Option<Registered>> {
    GPU_RAM_WITNESS.lock().unwrap_or_else(|e| e.into_inner())
}

fn planner() -> Result<*mut crate::gpu_bindings::CountAndPlanHandle, String> {
    Ok(registry().as_ref().ok_or("no GPU planner registered for the memory witness")?.inner)
}

// ─── Air layouts ─────────────────────────────────────────────────────────────────────────────

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

fn mem_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{MemTrace, MemTraceRowPacked};
    let widths = packed_widths(MemTrace::<()>::AIRGROUP_ID, MemTrace::<()>::AIR_ID)?;
    let words = MemTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::mem_lanes_x_row() as u32;
    Some((widths, words, lanes))
}

fn rom_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{RomDataTrace, RomDataTraceRowPacked};
    let widths = packed_widths(RomDataTrace::<()>::AIRGROUP_ID, RomDataTrace::<()>::AIR_ID)?;
    let words = RomDataTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::rom_data_lanes_x_row() as u32;
    Some((widths, words, lanes))
}

fn input_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{InputDataTrace, InputDataTraceRowPacked};
    let widths = packed_widths(InputDataTrace::<()>::AIRGROUP_ID, InputDataTrace::<()>::AIR_ID)?;
    let words = InputDataTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::input_data_lanes_x_row() as u32;
    Some((widths, words, lanes))
}

/// The planner's MemAlign kind of an air: 0 full, 1 byte, 2 read byte, 3 write byte.
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

/// Registers the runner's planner for the block and hands it the packed layouts of the memory
/// airs. Returns the families whose layout the planner took (`MEM_ROWS_*` bits); a family left
/// out keeps the CPU witness.
pub fn register_gpu_ram_witness(planner: &crate::GpuCountAndPlan, chunk_size: u64) -> u32 {
    let inner = planner.raw_handle();
    let Some((widths, words, lanes)) = mem_layout() else {
        tracing::warn!("[gpu] Mem air is not packed; the memory witness stays on the CPU");
        return 0;
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
            "[gpu] Mem layout rejected by the planner; the memory witness stays on the CPU"
        );
        return 0;
    }
    let mut families = zisk_common::MEM_ROWS_RAM;
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
    if rom_ok {
        families |= zisk_common::MEM_ROWS_ROM;
    } else {
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
    if input_ok {
        families |= zisk_common::MEM_ROWS_INPUT;
    } else {
        tracing::warn!(
            "[gpu] InputData layout rejected by the planner; its witness stays on the CPU"
        );
    }
    let mut align_ok = true;
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
        align_ok &= ok;
    }
    if align_ok {
        families |= zisk_common::MEM_ROWS_ALIGN;
    } else {
        tracing::warn!(
            "[gpu] a MemAlign layout was rejected by the planner; those airs stay on the CPU"
        );
    }
    *registry() = Some(Registered { inner });
    families
}

/// Forgets the registered planner (the next block is starting), once no call still uses it.
pub fn clear_gpu_ram_witness() {
    while IN_FLIGHT.load(Ordering::Acquire) > 0 {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    *registry() = None;
    ASYNC_PREP.store(false, Ordering::Release);
}

// ─── The block's preparation ─────────────────────────────────────────────────────────────────

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
        let n_rows = zisk_sm_mem_common::mem_align_air_rows(plan.air_id)
            .ok_or("MemAlign air without rows")?;
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

/// The block's preparation, running on its own thread; joined by the first consumer that needs
/// it complete, or by the arena's release.
struct SlotPrepare {
    handle: Option<std::thread::JoinHandle<Result<RamFillPrepared, String>>>,
    done: Option<Result<RamFillPrepared, String>>,
}
static SLOT_PREPARE: Mutex<SlotPrepare> = Mutex::new(SlotPrepare { handle: None, done: None });

/// The block's rows are still on the device (the preparation runs or its images stand): the
/// fills take them from there. Cleared when the arena goes back; the fills then take the pinned
/// host copy.
static ASYNC_PREP: AtomicBool = AtomicBool::new(false);

/// Starts the block's preparation on its own thread, off the executor's path: it runs beside the
/// registration of the secondaries and the first commits, and each instance waits only for its
/// own image. The host-side checks stay synchronous, so a block the device cannot serve at all
/// falls back before any instance is built. With `host_rows` (slot mode) the owned instances'
/// rows are also copied to pinned host memory for the proofs.
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
    // A preparation still running (a block that ended early) is joined before a new one starts:
    // two never run on one planner.
    {
        let mut g = SLOT_PREPARE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(stale) = g.handle.take() {
            let _ = stale.join();
        }
        g.done = None;
    }
    ASYNC_PREP.store(true, Ordering::Release);
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
            tracing::debug!(
                "[gpu] memory witness prepared on the device: {} accesses resolved, {} instances, families {:#x}",
                prepared.n_accesses,
                prepared.n_instances,
                prepared.families
            );
            Ok(prepared)
        })
        .map_err(|e| {
            ASYNC_PREP.store(false, Ordering::Release);
            format!("cannot start the slot preparation thread: {e}")
        })?;
    SLOT_PREPARE.lock().unwrap_or_else(|e| e.into_inner()).handle = Some(handle);
    Ok(())
}

/// Waits for the block's preparation; its failure is every fill's failure. The lock is held
/// through the join, so a second waiter blocks until the result is known.
fn slot_prepare_join() -> Result<RamFillPrepared, String> {
    let mut g = SLOT_PREPARE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = g.handle.take() {
        g.done = Some(match h.join() {
            Ok(r) => r,
            Err(_) => Err("the slot preparation thread panicked".into()),
        });
    }
    g.done.clone().unwrap_or(Err("no memory witness preparation for this block".into()))
}

/// The families the preparation serves (`MEM_ROWS_*` bits), once it ended; 0 when it failed.
pub fn gpu_mem_witness_families() -> u32 {
    slot_prepare_join().map(|p| p.families).unwrap_or(0)
}

/// `MEM_ROWS_*` bits of the families whose owned instances all have their rows in pinned host
/// memory (zero before the preparation ends, or without host rows).
pub fn gpu_mem_witness_host_rows_mask() -> u32 {
    if slot_prepare_join().is_err() {
        return 0;
    }
    match planner() {
        // SAFETY: registered handle; the mask is a plain read of the planner's state.
        Ok(inner) => unsafe { crate::gpu_bindings::count_and_plan_host_rows_mask(inner) },
        Err(_) => 0,
    }
}

// ─── The arena's release ─────────────────────────────────────────────────────────────────────

/// What runs when the last owned memory instance took its rows: the arena goes back to the
/// prover, and the memory airs leave the device for the rest of the block.
struct SlotRelease {
    pending: usize,
    release: Option<Box<dyn FnOnce() + Send>>,
}
static SLOT_RELEASE: Mutex<SlotRelease> = Mutex::new(SlotRelease { pending: 0, release: None });

/// Arms the release that follows the last of `n_pending` fills.
pub fn gpu_slot_witness_arm(n_pending: usize, release: Box<dyn FnOnce() + Send>) {
    let mut g = SLOT_RELEASE.lock().unwrap_or_else(|e| e.into_inner());
    g.pending = n_pending;
    g.release = Some(release);
}

/// One fill ended (with rows or not): the release to run when it was the last pending one.
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

/// Runs `release` after the preparation ended and every copy out of the arena landed; the rows
/// then come from the host.
fn run_release(release: Box<dyn FnOnce() + Send>) {
    let _ = slot_prepare_join();
    if let Ok(inner) = planner() {
        // SAFETY: registered handle; the planner waits for its own streams.
        unsafe { crate::gpu_bindings::count_and_plan_slot_quiesce(inner) };
    }
    ASYNC_PREP.store(false, Ordering::Release);
    release();
}

/// Runs the armed release now, if it has not run: the end of the proof, a block whose memory
/// instances were not all computed here, or a failure that sends the block to the CPU witness.
pub fn gpu_slot_witness_release_now() {
    let release = {
        let mut g = SLOT_RELEASE.lock().unwrap_or_else(|e| e.into_inner());
        g.pending = 0;
        g.release.take()
    };
    match release {
        Some(release) => run_release(release),
        None => ASYNC_PREP.store(false, Ordering::Release),
    }
}

/// Counts one fill toward the release on every exit, rows or error, so the arena always goes
/// back; also keeps the registration alive for the call.
struct FillGuard;
impl FillGuard {
    fn enter() -> Self {
        IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
        Self
    }
}
impl Drop for FillGuard {
    fn drop(&mut self) {
        if let Some(release) = slot_fill_done() {
            run_release(release);
        }
        IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    }
}

// ─── Rows ────────────────────────────────────────────────────────────────────────────────────

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

/// The pinned host rows of an instance and its scalars, once the preparation wrote them.
///
/// # Safety
/// `inner` must be the registered planner, held under the registry lock by the caller; the
/// returned slice points into the planner's pinned buffer, valid until the next block (reset),
/// which cannot start before this witness phase ends.
unsafe fn pinned_rows(
    inner: *mut crate::gpu_bindings::CountAndPlanHandle,
    family: u32,
    air_id: u32,
    segment: u32,
    len: usize,
) -> Option<(&'static [u64], RamFillResult)> {
    use crate::gpu_bindings as b;
    let mut res = RamFillResult::default();
    // SAFETY: registered handle under the caller's lock; `res` is a valid out-parameter.
    let p = unsafe {
        match family {
            MEM_FAMILY_RAM => b::count_and_plan_ram_instance_rows(inner, segment, &mut res),
            MEM_FAMILY_ROM => b::count_and_plan_rom_instance_rows(inner, segment, &mut res),
            MEM_FAMILY_INPUT => b::count_and_plan_input_instance_rows(inner, segment, &mut res),
            MEM_FAMILY_ALIGN => {
                b::count_and_plan_align_instance_rows(inner, air_id, segment, &mut res)
            }
            _ => std::ptr::null(),
        }
    };
    if p.is_null() {
        return None;
    }
    // SAFETY: the buffer holds `len` words for this instance (the planner sized it from the same
    // layout the caller checked the length against).
    Some((unsafe { std::slice::from_raw_parts(p, len) }, res))
}

/// The rows of one memory instance (`family` 0 Mem, 1 RomData, 2 InputData, 3 MemAlign by
/// `air_id`) into `out_rows` (`n_rows` packed rows). While the block's rows are on the device the
/// instance waits for its own staged image and copies it, or has its rows built from the retained
/// accesses when none was staged; once the arena went back it copies the pinned host rows. Counts
/// toward the arena's release either way.
fn rows_into(
    family: u32,
    air_id: u32,
    segment: u32,
    n_rows: u32,
    out_rows: &mut [u64],
) -> Result<RamFillResult, String> {
    let _guard = FillGuard::enter();
    let inner = planner()?;
    if !ASYNC_PREP.load(Ordering::Acquire) {
        // The proofs: the pinned copy the preparation left for this instance.
        let reg = registry();
        reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
        // SAFETY: registered handle, under the lock; the length is the caller's buffer.
        let (src, res) = unsafe { pinned_rows(inner, family, air_id, segment, out_rows.len()) }
            .ok_or_else(|| {
                format!(
                    "family {family} air {air_id} segment {segment}: no host rows for the proof"
                )
            })?;
        copy_rows(out_rows, src);
        return Ok(res);
    }
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
    match staged {
        Some(res) => Ok(res),
        None => {
            // Not staged (no room, or the image was dropped): the preparation's outcome first, then
            // the rows from the host copy or built into host memory, serialised with every other
            // planner call.
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
            Ok(res)
        }
    }
}

fn check_len(out_len: usize, n_rows: u32, words: u32) -> Result<(), String> {
    let len = n_rows as usize * words as usize;
    if out_len != len {
        return Err(format!("out_rows holds {out_len} words, {n_rows} rows need {len}"));
    }
    Ok(())
}

/// The rows of Mem instance `inst` into `out_rows` (`n_rows` rows of the packed layout).
pub fn gpu_ram_witness_fill(
    inst: u32,
    out_rows: &mut [u64],
    n_rows: u32,
) -> Result<RamFillResult, String> {
    check_len(out_rows.len(), n_rows, mem_layout().ok_or("Mem air is not packed")?.1)?;
    rows_into(MEM_FAMILY_RAM, 0, inst, n_rows, out_rows)
}

/// The rows of RomData instance `inst` into `out_rows`.
pub fn gpu_rom_witness_fill(
    inst: u32,
    out_rows: &mut [u64],
    n_rows: u32,
) -> Result<RamFillResult, String> {
    check_len(out_rows.len(), n_rows, rom_layout().ok_or("RomData air is not packed")?.1)?;
    rows_into(MEM_FAMILY_ROM, 0, inst, n_rows, out_rows)
}

/// The rows of InputData instance `inst` into `out_rows`.
pub fn gpu_input_witness_fill(
    inst: u32,
    out_rows: &mut [u64],
    n_rows: u32,
) -> Result<RamFillResult, String> {
    check_len(out_rows.len(), n_rows, input_layout().ok_or("InputData air is not packed")?.1)?;
    rows_into(MEM_FAMILY_INPUT, 0, inst, n_rows, out_rows)
}

/// The rows of MemAlign instance (`air_id`, `segment`) into `out_rows`; returns the rows its
/// accesses use.
pub fn gpu_align_witness_fill(
    air_id: usize,
    segment: usize,
    out_rows: &mut [u64],
) -> Result<usize, String> {
    let kind = align_air_kind(air_id).ok_or("not a MemAlign air")?;
    let words = align_layout(kind).ok_or("MemAlign air is not packed")?.1;
    let n_rows =
        zisk_sm_mem_common::mem_align_air_rows(air_id).ok_or("MemAlign air without rows")?;
    check_len(out_rows.len(), n_rows as u32, words)?;
    rows_into(MEM_FAMILY_ALIGN, air_id as u32, segment as u32, n_rows as u32, out_rows)
        .map(|r| r.n_lanes as usize)
}

/// The scalars of a resolved instance (`family` 0 Mem, 1 RomData, 2 InputData), for its air
/// values; valid after the preparation, with or without rows.
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

/// The rows a prepared MemAlign instance's accesses use, as the device built them.
pub fn gpu_mem_witness_align_used(air_id: usize, segment: usize) -> Result<u32, String> {
    slot_prepare_join()?;
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let mut used = 0u32;
    // SAFETY: registered handle, under the lock; `used` is a valid out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_align_used(
            r.inner,
            air_id as u32,
            segment as u32,
            &mut used,
        )
    };
    if !ok {
        return Err(format!(
            "MemAlign air {air_id} segment {segment} was not prepared on the device"
        ));
    }
    Ok(used)
}

// ─── The prover's kernel (slot mode) ─────────────────────────────────────────────────────────

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
    let _guard = FillGuard::enter();
    if let Err(e) = slot_prepare_join() {
        tracing::error!("[gpu] slot fill: the block's preparation failed: {e}");
        return -4;
    }
    let reg = registry();
    let Some(r) = reg.as_ref() else {
        tracing::error!("[gpu] slot fill: no GPU planner registered");
        return -1;
    };
    // SAFETY: registered handle, under the lock.
    let planner_device = unsafe { crate::gpu_bindings::count_and_plan_device(r.inner) };
    if device_id != planner_device {
        tracing::error!(
            "[gpu] slot fill: the slot is on GPU {device_id} but the planner holds the accesses on GPU {planner_device}; the memory airs must commit on the planner's GPU"
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
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    /// The release state is global: the tests take turns.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// The release runs exactly once, after the last of the armed fills, whatever path they exit by.
    #[test]
    fn release_fires_after_the_last_fill_and_only_once() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let fired = Arc::new(AtomicUsize::new(0));
        let f = fired.clone();
        gpu_slot_witness_arm(
            3,
            Box::new(move || {
                f.fetch_add(1, Ordering::SeqCst);
            }),
        );
        drop(FillGuard::enter());
        assert_eq!(fired.load(Ordering::SeqCst), 0);
        drop(FillGuard::enter());
        assert_eq!(fired.load(Ordering::SeqCst), 0);
        drop(FillGuard::enter());
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        drop(FillGuard::enter());
        gpu_slot_witness_release_now();
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        assert_eq!(IN_FLIGHT.load(Ordering::SeqCst), 0);
    }

    /// A block that ends before every fill ran still hands the arena back.
    #[test]
    fn release_now_runs_a_pending_release() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let fired = Arc::new(AtomicUsize::new(0));
        let f = fired.clone();
        gpu_slot_witness_arm(
            2,
            Box::new(move || {
                f.fetch_add(1, Ordering::SeqCst);
            }),
        );
        drop(FillGuard::enter());
        gpu_slot_witness_release_now();
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        drop(FillGuard::enter());
        assert_eq!(fired.load(Ordering::SeqCst), 1);
    }
}
