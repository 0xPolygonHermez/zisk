#![cfg(feature = "gpu")]

use std::os::raw::c_void;

use crate::gpu_bindings;

pub use crate::gpu_bindings::{InstanceMeta as GpuInstanceMeta, MemOp as GpuMemOp};

/// Safe Rust wrapper around the GPU `CountAndPlan` C++ class.
///
/// Mirrors the shape of [`crate::MemPlanner`] (CPU) so call-sites can be
/// switched between backends with minimal churn. Lifecycle:
///   1. `new()`            — allocates the GPU class
///   2. `setup(...)`       — initialize buffers / pick worker slice
///   3. `add_chunk(...)`   — feed memops, one chunk at a time
///   4. `run()`            — drains streams, returns borrowed metas
///   5. drop               — destroys the GPU class
pub struct GpuMemPlanner {
    inner: *mut gpu_bindings::CountAndPlanHandle,
}

unsafe impl Send for GpuMemPlanner {}
unsafe impl Sync for GpuMemPlanner {}

impl Default for GpuMemPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuMemPlanner {
    pub fn new() -> Self {
        let ptr = unsafe { gpu_bindings::count_and_plan_create() };
        assert!(!ptr.is_null(), "count_and_plan_create returned null");
        Self { inner: ptr }
    }

    /// Pass `d_buf = null, bytes = 0` to let the GPU class allocate internally.
    /// n_worker = 1 and worker_id = 0 to get all the instances metas otherwise distributed per worker_id using round-robin.
    pub fn setup(
        &self,
        d_buf: *mut c_void,
        bytes: usize,
        n_workers: u32,
        worker_id: u32,
    ) -> bool {
        unsafe { gpu_bindings::count_and_plan_setup(self.inner, d_buf, bytes, n_workers, worker_id) }
    }

    pub fn add_chunk(&self, memops: &[GpuMemOp]) -> bool {
        unsafe {
            gpu_bindings::count_and_plan_add_chunk(self.inner, memops.as_ptr(), memops.len() as u32)
        }
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
}

impl Drop for GpuMemPlanner {
    fn drop(&mut self) {
        unsafe { gpu_bindings::count_and_plan_destroy(self.inner) };
    }
}
