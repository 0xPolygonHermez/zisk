#![cfg(gpu)]

use std::os::raw::c_void;
use std::sync::Arc;

use mem_common::{MemAlignCounters, MemAlignPlanner};
use zisk_common::Plan;

use crate::gpu_bindings;

pub use crate::gpu_bindings::{
    GpuMemAlignCounter, InstanceMeta as GpuInstanceMeta, MemOp as GpuMemOp,
};

/// Serialize GPU-produced metas to `path` in the canonical `metas.bin`
/// format (the one `load_instance_metas` / the standalone runner use).
///
/// `metas` must be a slice returned by [`GpuCountAndPlan::run`] that is
/// still valid (the owning planner has not been `reset` or dropped). Intended
/// for capturing a block to share/debug, not for the hot path. Returns
/// `false` if `path` is not a valid C string or the FFI call fails.
pub fn save_gpu_metas(metas: &[GpuInstanceMeta], path: &str) -> bool {
    let Ok(c_path) = std::ffi::CString::new(path) else {
        return false;
    };
    unsafe {
        gpu_bindings::count_and_plan_save_metas(
            metas.as_ptr(),
            metas.len() as u32,
            c_path.as_ptr(),
        )
    }
}

/// Safe Rust wrapper around the C++ `CountAndPlan` GPU pipeline.
///
/// NOT an alternative to [`crate::MemPlanner`]. This is a *segment
/// producer*: it computes the planner `InstanceMeta[]` (and per-chunk
/// mem-align counters) on the GPU; the caller injects those into a
/// `MemPlanner`, which remains the owner of the segment table and the
/// final plan collector. Lifecycle:
///   1. `new()`            — allocates the GPU class
///   2. `setup(...)`       — initialize buffers / pick worker slice
///   3. `add_chunk(...)`   — feed memops, one chunk at a time
///   4. `run()`            — drains streams, returns borrowed metas
///   5. `reset()`          — reused across blocks; never recreated
///                           (recreation is ~240 ms of CUDA churn)
///   6. drop               — destroys the GPU class
pub struct GpuCountAndPlan {
    inner: *mut gpu_bindings::CountAndPlanHandle,
}

unsafe impl Send for GpuCountAndPlan {}
unsafe impl Sync for GpuCountAndPlan {}

impl Default for GpuCountAndPlan {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuCountAndPlan {
    pub fn new() -> Self {
        let ptr = unsafe { gpu_bindings::count_and_plan_create() };
        assert!(!ptr.is_null(), "count_and_plan_create returned null");
        Self { inner: ptr }
    }

    /// Pass `d_buf = null, bytes = 0` to let the GPU class allocate internally.
    /// n_worker = 1 and worker_id = 0 to get all the instances metas otherwise distributed per worker_id using round-robin.
    pub fn setup(&self, d_buf: *mut c_void, bytes: usize, n_workers: u32, worker_id: u32) -> bool {
        unsafe {
            gpu_bindings::count_and_plan_setup(self.inner, d_buf, bytes, n_workers, worker_id)
        }
    }

    pub fn add_chunk(&self, memops: &[GpuMemOp]) -> bool {
        unsafe {
            gpu_bindings::count_and_plan_add_chunk(self.inner, memops.as_ptr(), memops.len() as u32)
        }
    }

    /// Clear per-block state so the same planner instance can process the
    /// next block. Cheap — keeps the arena and per-stream resources alive.
    pub fn reset(&self) {
        unsafe { gpu_bindings::count_and_plan_reset(self.inner) };
    }

    pub fn register_input_pinned(&self, ptr: *const c_void, bytes: usize) -> bool {
        unsafe {
            gpu_bindings::count_and_plan_register_input_pinned(self.inner, ptr as *mut c_void, bytes)
        }
    }

    pub fn unregister_input_pinned(&self, ptr: *const c_void) {
        unsafe { gpu_bindings::count_and_plan_unregister_input_pinned(self.inner, ptr as *mut c_void) };
    }

    /// Returns a borrowed slice of metas owned by the C++ side. Valid until the next `reset` (or drop).
    pub fn run(&self) -> Option<&[GpuInstanceMeta]> {
        let mut ptr: *mut GpuInstanceMeta = std::ptr::null_mut();
        let mut n: u32 = 0;
        let ok = unsafe { gpu_bindings::count_and_plan_run(self.inner, &mut ptr, &mut n) };
        if !ok || ptr.is_null() {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(ptr, n as usize) })
    }

    /// Per-chunk mem-align counters produced by the count kernel. Index in the
    /// slice is the chunk_id. Valid after `run()`, until the next `reset()`.
    pub fn align_counters(&self) -> &[GpuMemAlignCounter] {
        let mut n: u32 = 0;
        let ptr = unsafe { gpu_bindings::count_and_plan_get_align_counters(self.inner, &mut n) };
        if ptr.is_null() || n == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(ptr, n as usize) }
    }

    /// Builds the mem-align plans straight from the GPU's per-chunk counters,
    /// replacing the CPU `MemPlanner::wait_mem_align_plans` path. Must be
    /// called after `run()`.
    pub fn build_align_plans(&self) -> Vec<Plan> {
        let gpu_counters = self.align_counters();
        let mut full_rows: u32 = 0;
        let mut read_byte: u32 = 0;
        let mut write_byte: u32 = 0;
        let counters: Vec<MemAlignCounters> = gpu_counters
            .iter()
            .enumerate()
            .map(|(i, c)| {
                full_rows += c.full_2 * 2 + c.full_3 * 3 + c.full_5 * 5;
                read_byte += c.read_byte;
                write_byte += c.write_byte;
                MemAlignCounters {
                    chunk_id: i as u32,
                    full_5: c.full_5,
                    full_3: c.full_3,
                    full_2: c.full_2,
                    read_byte: c.read_byte,
                    write_byte: c.write_byte,
                }
            })
            .collect();
        if counters.is_empty() {
            return Vec::new();
        }
        let mut planner = MemAlignPlanner::new(Arc::new(vec![]));
        planner.align_plan_from_counters(full_rows, read_byte, write_byte, &counters);
        planner.collect_plans()
    }
}

impl Drop for GpuCountAndPlan {
    fn drop(&mut self) {
        unsafe { gpu_bindings::count_and_plan_destroy(self.inner) };
    }
}
