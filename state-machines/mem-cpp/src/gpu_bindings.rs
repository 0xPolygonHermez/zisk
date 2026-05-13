#![cfg(gpu)]

use std::os::raw::c_void;

#[repr(C, align(8))]
#[derive(Copy, Clone, Debug)]
pub struct MemOp {
    pub addr: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceMeta {
    pub inst_id: u32,
    pub kind: u32,
    pub first_addr: u32,
    pub last_addr: u32,
    pub count_per_chunk: *const u32,
    pub n_chunks: u32,
    pub addr_offsets: *const u32,
    pub addr_offsets_size: u32,
    pub first_addr_chunk: u32,
    pub first_addr_skip: u32,
    pub last_addr_chunk: u32,
    pub last_addr_include: u32,
}

pub enum CountAndPlanHandle {}

/// Per-chunk mem-align counters produced by the GPU kernel. Same five u32
/// fields the CPU planner's `MemAlignCounters` uses (without `chunk_id` —
/// the index in the returned slice IS the chunk_id).
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct GpuMemAlignCounter {
    pub full_5: u32,
    pub full_3: u32,
    pub full_2: u32,
    pub read_byte: u32,
    pub write_byte: u32,
}

extern "C" {
    pub fn count_and_plan_create() -> *mut CountAndPlanHandle;
    pub fn count_and_plan_destroy(h: *mut CountAndPlanHandle);
    pub fn count_and_plan_setup(
        h: *mut CountAndPlanHandle,
        d_buf: *mut c_void,
        bytes: usize,
        n_workers: u32,
        worker_id: u32,
    ) -> bool;
    pub fn count_and_plan_add_chunk(
        h: *mut CountAndPlanHandle,
        memops: *const MemOp,
        n: u32,
    ) -> bool;
    pub fn count_and_plan_run(
        h: *mut CountAndPlanHandle,
        metas_out: *mut *mut InstanceMeta,
        n_metas: *mut u32,
    ) -> bool;
    pub fn count_and_plan_reset(h: *mut CountAndPlanHandle);
    pub fn count_and_plan_get_align_counters(
        h: *mut CountAndPlanHandle,
        n_chunks: *mut u32,
    ) -> *const GpuMemAlignCounter;
}
