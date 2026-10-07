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

/// Mirrors `AlignPlanDesc` in `cu/count_and_plan.cuh`: one MemAlign instance of the host plan.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct AlignPlanDesc {
    pub air_kind: u32,
    pub air_id: u32,
    pub segment: u32,
    pub n_rows: u32,
    pub entry_from: u32,
    pub entry_n: u32,
}
const _: () = assert!(core::mem::size_of::<AlignPlanDesc>() == 24);

/// Mirrors `AlignChunkEntry`: a chunk's per-kind windows (full_5, full_3, full_2, read_byte,
/// write_byte) of one instance.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct AlignChunkEntry {
    pub chunk: u32,
    pub skip: [u32; 5],
    pub count: [u32; 5],
}
const _: () = assert!(core::mem::size_of::<AlignChunkEntry>() == 44);

/// Mirrors `MemSlotOp` in `cu/count_and_plan.cuh`: the kernel input of one memory instance filled
/// into a prover slot. `family`: 0 Mem, 1 RomData, 2 InputData, 3 MemAlign.
/// A row image the preparation staged on the device (mirrors `StagedRows` in count_and_plan.cuh).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct StagedRows {
    pub ptr: *const u64,
    pub words: u64,
    pub family: u32,
    pub air_id: u32,
    pub segment: u32,
    pub n_rows: u32,
    pub res: RamFillResult,
}
impl Default for StagedRows {
    fn default() -> Self {
        Self { ptr: std::ptr::null(), words: 0, family: 0, air_id: 0, segment: 0, n_rows: 0, res: RamFillResult::default() }
    }
}
const _: () = assert!(std::mem::size_of::<StagedRows>() == 88);

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct MemSlotOp {
    pub family: u32,
    pub air_id: u32,
    pub segment: u32,
    pub n_rows: u32,
}
const _: () = assert!(core::mem::size_of::<MemSlotOp>() == 16);

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
        retain_rows: bool,
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
        insts: *const u32,
        n_insts: u32,
        prepared: *mut RamFillPrepared,
    ) -> bool;
    pub fn count_and_plan_set_rom_layout(
        h: *mut CountAndPlanHandle,
        col_widths: *const u32,
        n_cols: u32,
        words_per_row: u32,
        lanes_x_row: u32,
    ) -> bool;
    pub fn count_and_plan_fill_all_rom_instances(
        h: *mut CountAndPlanHandle,
        n_rows: u32,
        insts: *const u32,
        n_insts: u32,
        prepared: *mut RamFillPrepared,
    ) -> bool;
    pub fn count_and_plan_rom_instance_rows(
        h: *mut CountAndPlanHandle,
        inst: u32,
        res: *mut RamFillResult,
    ) -> *const u64;
    pub fn count_and_plan_set_input_layout(
        h: *mut CountAndPlanHandle,
        col_widths: *const u32,
        n_cols: u32,
        words_per_row: u32,
        lanes_x_row: u32,
    ) -> bool;
    pub fn count_and_plan_fill_all_input_instances(
        h: *mut CountAndPlanHandle,
        n_rows: u32,
        image: *const u8,
        image_bytes: usize,
        insts: *const u32,
        n_insts: u32,
        prepared: *mut RamFillPrepared,
    ) -> bool;
    pub fn count_and_plan_input_instance_rows(
        h: *mut CountAndPlanHandle,
        inst: u32,
        res: *mut RamFillResult,
    ) -> *const u64;
    pub fn count_and_plan_set_align_layout(
        h: *mut CountAndPlanHandle,
        air_kind: u32,
        col_widths: *const u32,
        n_cols: u32,
        words_per_row: u32,
    ) -> bool;
    pub fn count_and_plan_fill_all_align_instances(
        h: *mut CountAndPlanHandle,
        plans: *const AlignPlanDesc,
        n_plans: u32,
        entries: *const AlignChunkEntry,
        n_entries: u32,
        prepared: *mut RamFillPrepared,
    ) -> bool;
    pub fn count_and_plan_align_instance_rows(
        h: *mut CountAndPlanHandle,
        air_id: u32,
        segment: u32,
        res: *mut RamFillResult,
    ) -> *const u64;
    pub fn count_and_plan_prepare_slot_fills(
        h: *mut CountAndPlanHandle,
        image: *const u8,
        image_bytes: usize,
        plans: *const AlignPlanDesc,
        n_plans: u32,
        entries: *const AlignChunkEntry,
        n_entries: u32,
        prepared: *mut RamFillPrepared,
    ) -> bool;
    /// The device the planner holds the retained accesses on.
    pub fn count_and_plan_device(h: *mut CountAndPlanHandle) -> i32;
    pub fn count_and_plan_fill_slot(
        h: *mut CountAndPlanHandle,
        d_ops: *const core::ffi::c_void,
        n_ops: u64,
        dst: *mut u64,
        stream: *mut core::ffi::c_void,
        res: *mut RamFillResult,
    ) -> bool;
    /// Waits for the slot copies still in flight on the prover's streams.
    pub fn count_and_plan_slot_quiesce(h: *mut CountAndPlanHandle);
    /// 1 staged (`out` filled), 0 the preparation ended without it, -1 timeout. Safe to call while
    /// the preparation runs on another thread.
    pub fn count_and_plan_staged_wait(
        h: *mut CountAndPlanHandle,
        family: u32,
        air_id: u32,
        segment: u32,
        timeout_ms: u32,
        out: *mut StagedRows,
    ) -> i32;
    /// Copies a staged image (`words` words) to host memory on its own stream.
    pub fn count_and_plan_copy_staged(
        h: *mut CountAndPlanHandle,
        family: u32,
        air_id: u32,
        segment: u32,
        dst: *mut u64,
        words: u64,
    ) -> bool;
    /// Builds one instance's rows into host memory after the preparation (the registry lock).
    pub fn count_and_plan_fill_host(
        h: *mut CountAndPlanHandle,
        family: u32,
        air_id: u32,
        segment: u32,
        n_rows: u32,
        out_rows: *mut u64,
        res: *mut RamFillResult,
    ) -> bool;
    pub fn count_and_plan_instance_scalars(
        h: *mut CountAndPlanHandle,
        family: u32,
        inst: u32,
        res: *mut RamFillResult,
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
