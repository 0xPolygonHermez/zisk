use std::os::raw::c_void;

#[repr(C, align(8))]
#[derive(Copy, Clone, Debug)]
pub struct MemOp {
    pub addr: u32,
    pub flags: u32,
    /// Written value for non-block writes; step field for block records.
    pub payload: u64,
}
const _: () = assert!(core::mem::size_of::<MemOp>() == 16);

/// Paged cumulative-offset table. WIRE/FFI-significant: field order &
/// types must match `cpp/instance_meta.hpp::PagedOffsets` and the
/// per-field serialisation in `cpp/instance_meta_loader.hpp` exactly.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PagedOffsets {
    pub page_starts: *const u32,
    pub page_single_value: *const u32,
    pub pages_dense: *const u32,
    pub num_pages: u32,
    pub present_count: u32,
    pub addr_range_slots: u32,
}
// Catches an accidental add/remove of a field vs. the C++ POD (which
// carries the matching `static_assert(sizeof(PagedOffsets) == 40)`).
const _: () = assert!(core::mem::size_of::<PagedOffsets>() == 40);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceMeta {
    pub inst_id: u32,
    pub kind: u32,
    pub first_addr: u32,
    pub last_addr: u32,
    pub count_per_chunk: *const u32,
    pub n_chunks: u32,
    pub offsets: PagedOffsets,
    pub first_addr_chunk: u32,
    pub first_addr_skip: u32,
    pub last_addr_chunk: u32,
    pub last_addr_include: u32,
}

pub enum CountAndPlanHandle {}

/// Mirrors `RamFillPrepared` in `cu/count_and_plan.cuh`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RamFillPrepared {
    pub status: i32,
    pub n_instances: u32,
    pub n_accesses: u64,
    pub n_lanes: u64,
    pub ms_sort: f32,
    pub ms_lanes: f32,
    pub ms_values: f32,
    pub ms_total: f32,
}
const _: () = assert!(core::mem::size_of::<RamFillPrepared>() == 40);

/// Mirrors `RamFillResult` in `cu/count_and_plan.cuh`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RamFillResult {
    pub status: i32,
    pub n_lanes: u32,
    pub prev_addr_w: u32,
    pub last_addr_w: u32,
    pub prev_step: u64,
    pub prev_value: u64,
    pub last_step: u64,
    pub last_value: u64,
    pub ms_rows: f32,
    pub ms_d2h: f32,
}
const _: () = assert!(core::mem::size_of::<RamFillResult>() == 56);

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
        gpu_id: i32,
        instance_rows: *const u32,
    ) -> bool;
    /// `words`: the chunk's memory-ops stream, `n_words` tagged 8-byte words.
    pub fn count_and_plan_add_chunk(
        h: *mut CountAndPlanHandle,
        words: *const u64,
        n_words: u32,
    ) -> bool;
    pub fn count_and_plan_run(
        h: *mut CountAndPlanHandle,
        metas_out: *mut *mut InstanceMeta,
        n_metas: *mut u32,
    ) -> bool;
    pub fn count_and_plan_reset(h: *mut CountAndPlanHandle);
    pub fn count_and_plan_set_chunk_size_bits(h: *mut CountAndPlanHandle, bits: u32);
    pub fn count_and_plan_set_mem_layout(
        h: *mut CountAndPlanHandle,
        col_widths: *const u32,
        n_cols: u32,
        words_per_row: u32,
        lanes_x_row: u32,
    ) -> bool;
    pub fn count_and_plan_ram_retention_ok(h: *mut CountAndPlanHandle) -> bool;
    pub fn count_and_plan_prepare_ram_fill(
        h: *mut CountAndPlanHandle,
        out: *mut RamFillPrepared,
    ) -> bool;
    pub fn count_and_plan_fill_all_ram_instances(
        h: *mut CountAndPlanHandle,
        n_rows: u32,
        prepared: *mut RamFillPrepared,
    ) -> bool;
    pub fn count_and_plan_ram_instance_rows(
        h: *mut CountAndPlanHandle,
        inst: u32,
        res: *mut RamFillResult,
    ) -> *const u64;
    pub fn count_and_plan_max_used_bytes(h: *mut CountAndPlanHandle) -> usize;
    pub fn count_and_plan_register_input_pinned(
        h: *mut CountAndPlanHandle,
        ptr: *mut c_void,
        bytes: usize,
    ) -> bool;
    pub fn count_and_plan_unregister_input_pinned(h: *mut CountAndPlanHandle, ptr: *mut c_void);
    pub fn count_and_plan_get_align_counters(
        h: *mut CountAndPlanHandle,
        n_chunks: *mut u32,
    ) -> *const GpuMemAlignCounter;
    pub fn count_and_plan_save_metas(
        metas: *const InstanceMeta,
        n: u32,
        path: *const std::os::raw::c_char,
    ) -> bool;
}
