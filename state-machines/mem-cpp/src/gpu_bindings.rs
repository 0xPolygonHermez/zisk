#![cfg(feature = "gpu")]

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
}
