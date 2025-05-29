use std::os::raw::c_void;

use crate::*;

pub struct MemPlanner {
    inner: *mut bindings::MemCountAndPlan,
}

impl Default for MemPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl MemPlanner {
    /// Creates and prepares the planner
    pub fn new() -> Self {
        let ptr = unsafe { bindings::create_mem_count_and_plan() };
        assert!(!ptr.is_null(), "Failed to create MemCountAndPlan");
        Self { inner: ptr }
    }

    /// Starts execution (spawns internal threads)
    pub fn execute(&self) {
        unsafe { bindings::execute_mem_count_and_plan(self.inner) };
    }

    /// Adds a chunk of memory data
    pub fn add_chunk(&self, len: u64, data: *const c_void) {
        unsafe {
            bindings::add_chunk_mem_count_and_plan(
                self.inner,
                data as *mut bindings::MemCountersBusData,
                len as u32,
            );
        }
    }

    pub fn stats(&self) {
        unsafe { bindings::stats_mem_count_and_plan(self.inner) };
    }

    /// Signals completion
    pub fn set_completed(&self) {
        unsafe { bindings::set_completed_mem_count_and_plan(self.inner) };
    }

    /// Waits for all background processing to complete
    pub fn wait(&self) {
        unsafe { bindings::wait_mem_count_and_plan(self.inner) };
    }
}

impl Drop for MemPlanner {
    fn drop(&mut self) {
        unsafe {
            bindings::destroy_mem_count_and_plan(self.inner);
        }
    }
}
