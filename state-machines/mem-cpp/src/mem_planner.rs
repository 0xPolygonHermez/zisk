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

    pub fn inner(&self) -> *mut bindings::MemCountAndPlan {
        self.inner
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

    /// Retrieves a Vec of MemCheckpoint from C++ given a valid segment ID.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements.
    pub fn get_segments(&self) -> (Vec<Vec<MemCheckPoint>>, Vec<Vec<MemAlignCheckPoint>>) {
        let mem_segments_count: u32 = unsafe { bindings::get_mem_segment_count(self.inner) };

        let mut mem_segments = Vec::new();
        for segment_id in 0..mem_segments_count {
            mem_segments.push(MemCheckPoint::from_cpp(self, segment_id));
        }

        let mem_align_segments_count: u32 =
            unsafe { bindings::get_mem_align_segment_count(self.inner) };

        let mut mem_align_segments = Vec::new();
        for segment_id in 0..mem_align_segments_count {
            mem_align_segments.push(MemAlignCheckPoint::from_cpp(self, segment_id));
        }

        (mem_segments, mem_align_segments)
    }
}

impl Drop for MemPlanner {
    fn drop(&mut self) {
        unsafe {
            bindings::destroy_mem_count_and_plan(self.inner);
        }
    }
}
