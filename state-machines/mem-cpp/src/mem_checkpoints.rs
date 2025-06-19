use crate::{bindings, MemPlanner};

/// Represents a memory checkpoint
#[repr(C)]
#[derive(Debug)]
pub struct MemCheckPoint {
    chunk_id: u32,
    from_addr: u32,
    from_skip: u32,
    to_addr: u32,
    to_count: u32,
    count: u32,
    is_first_chunk: u32,
}

impl MemCheckPoint {
    pub fn new(
        chunk_id: u32,
        from_addr: u32,
        from_skip: u32,
        to_addr: u32,
        to_count: u32,
        count: u32,
        is_first_chunk: u32,
    ) -> Self {
        Self { chunk_id, from_addr, from_skip, to_addr, to_count, count, is_first_chunk }
    }

    /// Retrieves a Vec of MemCheckpoint from C++ given a valid segment ID.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements.
    pub fn from_cpp(mem_planner: &MemPlanner, segment_id: u32) -> Vec<MemCheckPoint> {
        let mut count: u32 = 0;

        let ptr = unsafe {
            bindings::get_mem_segment_check_point(
                mem_planner.inner(),
                segment_id,
                &mut count as *mut u32,
            )
        } as *mut MemCheckPoint;

        if ptr.is_null() || count == 0 {
            return Vec::new();
        }

        // SAFETY: assumes pointer is valid for `count` elements
        unsafe { Vec::from_raw_parts(ptr, count as usize, count as usize) }
    }
}

/// Represents a memory alignment checkpoint.
#[repr(C)]
#[derive(Debug)]
pub struct MemAlignCheckPoint {
    chunk_id: u32,
    skip: u32,
    count: u32,
    rows: u32,
}

impl MemAlignCheckPoint {
    pub fn new(chunk_id: u32, skip: u32, count: u32, rows: u32) -> Self {
        Self { chunk_id, skip, count, rows }
    }

    /// Retrieves a Vec of MemAlignCheckPoint from C++ given a valid segment ID.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements.
    pub fn from_cpp(mem_planner: &MemPlanner, segment_id: u32) -> Vec<MemAlignCheckPoint> {
        let mut count: u32 = 0;

        let ptr = unsafe {
            bindings::get_mem_align_segment_check_point(
                mem_planner.inner(),
                segment_id,
                &mut count as *mut u32,
            )
        } as *mut MemAlignCheckPoint;

        if ptr.is_null() || count == 0 {
            return Vec::new();
        }

        // SAFETY: assumes pointer is valid for `count` elements
        unsafe { Vec::from_raw_parts(ptr, count as usize, count as usize) }
    }
}
