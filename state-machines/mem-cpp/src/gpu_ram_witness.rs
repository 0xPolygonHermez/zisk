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

/// Packed column widths, words per row and lanes of the `Mem` air.
fn mem_layout() -> Option<(Vec<u32>, u32, u32)> {
    use zisk_pil::{MemTrace, MemTraceRowPacked, PACKED_INFO};
    let airgroup_id = MemTrace::<()>::AIRGROUP_ID;
    let air_id = MemTrace::<()>::AIR_ID;
    let info = PACKED_INFO
        .iter()
        .find(|(ag, a, _)| *ag == airgroup_id && *a == air_id)
        .map(|(_, _, c)| c)?;
    if !info.is_packed {
        return None;
    }
    let widths: Vec<u32> = info.unpack_info.iter().map(|&w| w as u32).collect();
    let words = MemTraceRowPacked::<proofman_fields::Goldilocks>::PACKED_WORDS as u32;
    let lanes = zisk_sm_mem_common::mem_lanes_x_row() as u32;
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
    *registry() = Some(Registered { inner });
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
    let (src, res) = {
        let reg = registry();
        let r = reg.as_ref().ok_or("no GPU planner registered for the RAM witness")?;
        let Some((_, words, _)) = mem_layout() else {
            return Err("Mem air is not packed".to_string());
        };
        let len = n_rows as usize * words as usize;
        if out_rows.len() != len {
            return Err(format!(
                "out_rows holds {} words, {n_rows} rows need {len}",
                out_rows.len()
            ));
        }
        let mut res = RamFillResult::default();
        // SAFETY: registered handle, under the lock; `res` is a valid out-parameter.
        let p = unsafe {
            crate::gpu_bindings::count_and_plan_ram_instance_rows(r.inner, inst, &mut res)
        };
        if p.is_null() {
            return Err(format!(
                "RAM instance {inst} has no rows: fill_all_ram_instances did not run or failed"
            ));
        }
        // SAFETY: the pinned rows buffer holds `len` words for this instance and stays valid until
        // the planner's next block (reset), which cannot start before this witness phase ends.
        (unsafe { std::slice::from_raw_parts(p, len) }, res)
    };
    // The lock is released: the copy is the only remaining work, and it is large.
    let threads = 8usize;
    let per = src.len().div_ceil(threads);
    std::thread::scope(|sc| {
        for (d, s) in out_rows.chunks_mut(per).zip(src.chunks(per)) {
            sc.spawn(move || d.copy_from_slice(s));
        }
    });
    Ok(res)
}
