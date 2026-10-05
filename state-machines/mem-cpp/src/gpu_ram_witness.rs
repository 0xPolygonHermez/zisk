//! The RAM witness the GPU planner can produce from the accesses it retained.
//!
//! The runner registers its planner here once the plan is closed; the `Mem` instances of the same
//! block then ask for their rows. The planner object lives in the runner's preloaded state, so it
//! outlives the witness phase; the registration is cleared when the next block starts.

use std::sync::{Mutex, MutexGuard};

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
    *registry() = Some(Registered { inner });
}

/// Builds every RomData instance's rows into pinned host memory, after the RAM fill and while the
/// planner's arena is still borrowed.
pub fn gpu_rom_witness_fill_all() -> Result<RamFillPrepared, String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the memory witness")?;
    let n_rows = zisk_pil::RomDataTrace::<()>::NUM_ROWS as u32;
    let mut prepared = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; `prepared` is a valid out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_fill_all_rom_instances(r.inner, n_rows, &mut prepared)
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

/// Builds every InputData instance's rows into pinned host memory, after the RomData fill. `image`
/// is the input region as the guest sees it, from its first byte.
pub fn gpu_input_witness_fill_all(image: &[u8]) -> Result<RamFillPrepared, String> {
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

/// Builds every RAM instance's rows into pinned host memory. Must run while the planner's arena is
/// still borrowed (right after the plan); the witness phase then only copies.
pub fn gpu_ram_witness_fill_all() -> Result<RamFillPrepared, String> {
    let reg = registry();
    let r = reg.as_ref().ok_or("no GPU planner registered for the RAM witness")?;
    let n_rows = zisk_pil::MemTrace::<()>::NUM_ROWS as u32;
    let mut prepared = RamFillPrepared::default();
    // SAFETY: registered handle, under the lock; `prepared` is a valid out-parameter.
    let ok = unsafe {
        crate::gpu_bindings::count_and_plan_fill_all_ram_instances(r.inner, n_rows, &mut prepared)
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
