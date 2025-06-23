use crate::{bindings, MemPlanner};

/// Represents a memory checkpoint
#[repr(C)]
#[derive(Debug)]
pub struct CppMemCheckPoint {
    pub chunk_id: u32,
    pub from_addr: u32,
    pub from_skip: u32,
    pub to_addr: u32,
    pub to_count: u32,
    pub count: u32,
}

impl CppMemCheckPoint {
    /// Retrieves a array pointer of MemCheckpoint from C++ given a valid segment ID.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements. The ownership of array remains with C++.
    pub fn from_cpp(mem_planner: &MemPlanner, mem_id: u32, segment_id: u32) -> &[CppMemCheckPoint] {
        let mut count: u32 = 0;

        let ptr = unsafe {
            bindings::get_mem_segment_check_points(
                mem_planner.inner(),
                mem_id,
                segment_id,
                &mut count as *mut u32,
            )
        } as *mut CppMemCheckPoint;

        if ptr.is_null() || count == 0 {
            return &[];
        }

        // SAFETY: assumes pointer is valid for `count` elements
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }
}

/// Represents a memory alignment checkpoint.
#[repr(C)]
#[derive(Debug)]
pub struct CppMemAlignCheckPoint {
    pub segment_id: u32,
    pub chunk_id: u32,
    pub skip: u32,
    pub count: u32,
    pub rows: u32,
    pub offset: u32, // row offset
}

impl CppMemAlignCheckPoint {
    // pub fn new(segmchunk_id: u32, skip: u32, count: u32, rows: u32) -> Self {
    //     Self { chunk_id, skip, count, rows }
    // }

    /// Retrieves a array pointer to all CppMemAlignCheckPoint from C++.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements. The ownership of array remains with C++.
    pub fn from_cpp(mem_planner: &MemPlanner) -> &[CppMemAlignCheckPoint] {
        let mut count: u32 = 0;

        let ptr = unsafe {
            bindings::get_mem_align_check_points(mem_planner.inner(), &mut count as *mut u32)
        } as *mut CppMemAlignCheckPoint;

        if ptr.is_null() || count == 0 {
            return &[];
        }

        // SAFETY: assumes pointer is valid for `count` elements
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }
}
