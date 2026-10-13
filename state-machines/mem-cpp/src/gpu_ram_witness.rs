//! The memory witness the GPU planner builds from the accesses it retained.
//!
//! The runner registers its planner here once the plan is closed and starts the block's
//! preparation on its own thread; the memory instances of the block then take their rows
//! (`gpu_*_witness_fill`) from the images the preparation staged on the device, or have them
//! built on demand, into their host trace buffers. The planner object lives in the runner's
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

/// Calls inside [`rows_into`]: the registration waits for them to end before it is cleared
/// (`clear_gpu_ram_witness`).
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
    ROWS_ON_DEVICE.store(false, Ordering::Release);
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
struct Prepare {
    handle: Option<std::thread::JoinHandle<Result<RamFillPrepared, String>>>,
    done: Option<Result<RamFillPrepared, String>>,
}
static PREPARE: Mutex<Prepare> = Mutex::new(Prepare { handle: None, done: None });

/// The block's rows are on the device (the preparation runs or its images stand): the fills take
/// them from there. Cleared when the arena goes back; a fill after that fails.
static ROWS_ON_DEVICE: AtomicBool = AtomicBool::new(false);

/// Starts the block's preparation on its own thread, off the executor's path: it runs beside the
/// registration of the secondaries and the first commits, and each instance waits only for its
/// own image. The host-side checks stay synchronous, so a block the device cannot serve at all
/// falls back before any instance is built. `ram`, `rom` and `input` (segment ids) and
/// `align_plans` are the owned instances, the ones that get a staged image.
pub fn gpu_mem_witness_prepare_async(
    image: Vec<u8>,
    ram: Vec<u32>,
    rom: Vec<u32>,
    input: Vec<u32>,
    align_plans: &[&zisk_common::Plan],
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
        let mut g = PREPARE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(stale) = g.handle.take() {
            let _ = stale.join();
        }
        g.done = None;
    }
    ROWS_ON_DEVICE.store(true, Ordering::Release);
    let handle = std::thread::Builder::new()
        .name("mem-witness-prepare".into())
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
                crate::gpu_bindings::count_and_plan_prepare_fills(
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
                    &mut prepared,
                )
            };
            if !ok {
                return Err(format!("prepare_fills failed with status {}", prepared.status));
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
            ROWS_ON_DEVICE.store(false, Ordering::Release);
            format!("cannot start the memory witness preparation thread: {e}")
        })?;
    PREPARE.lock().unwrap_or_else(|e| e.into_inner()).handle = Some(handle);
    Ok(())
}

/// Waits for the block's preparation; its failure is every fill's failure. The lock is held
/// through the join, so a second waiter blocks until the result is known.
fn prepare_join() -> Result<RamFillPrepared, String> {
    let mut g = PREPARE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = g.handle.take() {
        g.done = Some(match h.join() {
            Ok(r) => r,
            Err(_) => Err("the memory witness preparation thread panicked".into()),
        });
    }
    g.done.clone().unwrap_or(Err("no memory witness preparation for this block".into()))
}

// ─── The arena's release ─────────────────────────────────────────────────────────────────────

/// What runs when the last owned memory instance took its rows: the arena goes back to the
/// prover, and the memory airs leave the device for the rest of the block.
struct Release {
    pending: usize,
    release: Option<Box<dyn FnOnce() + Send>>,
}
static RELEASE: Mutex<Release> = Mutex::new(Release { pending: 0, release: None });

/// Arms the release that follows the last of `n_pending` fills.
pub fn gpu_mem_witness_arm(n_pending: usize, release: Box<dyn FnOnce() + Send>) {
    let mut g = RELEASE.lock().unwrap_or_else(|e| e.into_inner());
    g.pending = n_pending;
    g.release = Some(release);
}

/// One fill ended (with rows or not): the release to run when it was the last pending one.
fn fill_done() -> Option<Box<dyn FnOnce() + Send>> {
    let mut g = RELEASE.lock().unwrap_or_else(|e| e.into_inner());
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

/// Runs `release` after the preparation ended and every copy out of the arena landed.
fn run_release(release: Box<dyn FnOnce() + Send>) {
    let _ = prepare_join();
    {
        // Under the registry lock: a fill building rows from the arena holds it, so none is in
        // flight when the arena goes back.
        let reg = registry();
        if let Some(r) = reg.as_ref() {
            // SAFETY: registered handle; the planner waits for the copies still reading its images.
            unsafe { crate::gpu_bindings::count_and_plan_quiesce(r.inner) };
        }
    }
    ROWS_ON_DEVICE.store(false, Ordering::Release);
    release();
}

/// Runs the armed release now, if it has not run: the end of the proof, a block whose memory
/// instances were not all computed here, or a failure that sends the block to the CPU witness.
pub fn gpu_mem_witness_release_now() {
    let release = {
        let mut g = RELEASE.lock().unwrap_or_else(|e| e.into_inner());
        g.pending = 0;
        g.release.take()
    };
    match release {
        Some(release) => run_release(release),
        None => ROWS_ON_DEVICE.store(false, Ordering::Release),
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
        if let Some(release) = fill_done() {
            run_release(release);
        }
        IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    }
}

// ─── Rows ────────────────────────────────────────────────────────────────────────────────────

/// The rows of one memory instance (`family` 0 Mem, 1 RomData, 2 InputData, 3 MemAlign by
/// `air_id`) into `out_rows` (`n_rows` packed rows): the instance waits for its own staged image
/// and copies it, or has its rows built from the retained accesses when none was staged. Counts
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
    if !ROWS_ON_DEVICE.load(Ordering::Acquire) {
        return Err(format!(
            "family {family} air {air_id} segment {segment}: the block's rows are not on the device"
        ));
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
            // the rows built into host memory, serialised with every other planner call.
            prepare_join()?;
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
        gpu_mem_witness_arm(
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
        gpu_mem_witness_release_now();
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        assert_eq!(IN_FLIGHT.load(Ordering::SeqCst), 0);
    }

    /// A block that ends before every fill ran still hands the arena back.
    #[test]
    fn release_now_runs_a_pending_release() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let fired = Arc::new(AtomicUsize::new(0));
        let f = fired.clone();
        gpu_mem_witness_arm(
            2,
            Box::new(move || {
                f.fetch_add(1, Ordering::SeqCst);
            }),
        );
        drop(FillGuard::enter());
        gpu_mem_witness_release_now();
        assert_eq!(fired.load(Ordering::SeqCst), 1);
        drop(FillGuard::enter());
        assert_eq!(fired.load(Ordering::SeqCst), 1);
    }
}
