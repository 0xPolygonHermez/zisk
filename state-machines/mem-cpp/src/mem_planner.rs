use proofman_util::{timer_start_info, timer_stop_and_log_info};
use std::{os::raw::c_void, sync::Arc};

use crate::*;

#[cfg(feature = "save_mem_plans")]
use mem_common::save_plans;
use mem_common::MEM_OFFSETS_PAGE_SIZE;
use mem_common::{
    MemAlignCounters, MemAlignPlanner, MemModuleCheckPoint, MemModuleSegmentCheckPoint,
};

use zisk_common::{CheckPoint, ChunkId, InstanceType, Plan, SegmentId};
use zisk_pil::{INPUT_DATA_AIR_IDS, MEM_AIR_IDS, ROM_DATA_AIR_IDS, ZISK_AIRGROUP_ID};

pub struct MemPlanner {
    inner: *mut bindings::MemCountAndPlan,
}

unsafe impl Send for MemPlanner {}
unsafe impl Sync for MemPlanner {}

impl Default for MemPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// `MemPlanner` is a wrapper around a C++ memory planning and counting system, providing a safe Rust interface
/// for managing memory alignment, chunk addition, execution, and statistics collection. It manages the lifecycle
/// of the underlying C++ object, exposes methods to interact with memory chunks, retrieve alignment plans, and
/// collect memory usage statistics.
///
/// # Methods
/// - `new()`: Creates and prepares a new memory planner instance.
/// - `inner(&self)`: Returns a raw pointer to the underlying C++ planner object.
/// - `execute(&self)`: Starts execution, spawning internal threads for processing.
/// - `add_chunk(&self, len, data)`: Adds a chunk of memory data to the planner.
/// - `stats(&self)`: Prints or collects statistics from the planner.
/// - `set_completed(&self)`: Signals that all chunks have been added and processing can complete.
/// - `wait_mem_align_plans(&self)`: Waits for internal processing to finish and retrieves memory alignment plans.
/// - `wait(&self)`: Waits for all background processing to complete.
/// - `collect_plans(&self, mem_align_plans)`: Collects memory plans, incorporating provided alignment plans.
/// - `get_mem_align_counters(&self)`: Retrieves a slice of memory alignment counters (internal use).
/// - `get_total_mem_align_counters(&self)`: Retrieves the total memory alignment counters (internal use).
///
/// # Safety
/// Many methods interact with raw pointers and FFI bindings to C++ code. It is assumed that the underlying
/// memory is valid for the duration of the `MemPlanner` instance, and that the C++ side upholds its invariants.
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

    fn get_mem_align_counters(&self) -> &[MemAlignCounters] {
        let mut count: u32 = 0;

        let ptr = unsafe { bindings::get_mem_align_counters(self.inner(), &mut count as *mut u32) }
            as *mut MemAlignCounters;

        if ptr.is_null() || count == 0 {
            return &[];
        }

        // SAFETY: assumes pointer is valid for `count` elements
        unsafe { std::slice::from_raw_parts(ptr, count as usize) }
    }

    fn get_total_mem_align_counters(&self) -> MemAlignCounters {
        let ptr = unsafe { bindings::get_mem_align_total_counters(self.inner()) }
            as *mut MemAlignCounters;
        if ptr.is_null() {
            return MemAlignCounters::default();
        }
        unsafe { *ptr }
    }
    /// Retrieves a Vec of mem_align plans. This method first waits for the internal processing to complete and
    /// after that retrieves the plans.
    ///
    /// # Returns
    /// A vector of memory alignment plans.
    pub fn wait_mem_align_plans(&self) -> Vec<Plan> {
        unsafe {
            bindings::wait_mem_align_counters(self.inner);
        };
        let counters = self.get_mem_align_counters();
        let mut mem_align_planner = MemAlignPlanner::new(Arc::new(vec![]));

        // It is not necessary to calculate the totals; they have been accumulated as the counters were added
        let tot_counters = self.get_total_mem_align_counters();

        let full_rows = tot_counters.full_2 * 2 + tot_counters.full_3 * 3 + tot_counters.full_5 * 5;
        mem_align_planner.align_plan_from_counters(
            full_rows,
            tot_counters.read_byte,
            tot_counters.write_byte,
            counters,
        );
        mem_align_planner.collect_plans()
    }

    /// Waits for all background processing to complete
    pub fn wait(&self) {
        unsafe { bindings::wait_mem_count_and_plan(self.inner) };
    }

    /// Zero-copy injection of GPU-produced metas into `mcp->segments[]`.
    /// The GPU planner that produced these metas must remain alive across
    /// this call, since the per-meta `count_per_chunk` and `addr_offsets`
    /// pointers reference its pinned host memory.
    ///
    /// # Safety
    ///
    /// `gpu_metas` must point to a valid array of at least `n` meta entries,
    /// and the GPU planner that produced them must remain alive for the
    /// duration of this call (its pinned host memory is referenced directly).
    pub unsafe fn inject_gpu_metas_from_pointers(&self, gpu_metas: *const c_void, n: u32) -> bool {
        bindings::inject_gpu_metas_from_pointers(self.inner, gpu_metas, n)
    }

    /// Retrieves a Vec of memory plans, adding to this result plans the mem_align_plans provided as argument.
    ///
    /// # Parameters
    /// - `mem_align_plans`: A mutable reference to a vector of memory alignment plans to be moved to the result.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements.
    pub fn collect_plans(&self, mem_align_plans: &mut Vec<Plan>) -> Vec<Plan> {
        let mut plans = std::mem::take(mem_align_plans);
        timer_start_info!(COLLECT_MEM_PLANS);
        for (mem_id, air_id) in
            [ROM_DATA_AIR_IDS[0], INPUT_DATA_AIR_IDS[0], MEM_AIR_IDS[0]].iter().enumerate()
        {
            let mem_segments_count: u32 =
                unsafe { bindings::get_mem_segment_count(self.inner, mem_id as u32) };
            for segment_id in 0..mem_segments_count {
                let mut chunks: Vec<ChunkId> = Vec::new();
                let mut segment = MemModuleSegmentCheckPoint::new();
                segment.is_last_segment = segment_id == mem_segments_count - 1;
                let checkpoints = CppMemCheckPoint::from_cpp(self, mem_id as u32, segment_id);
                for checkpoint in checkpoints {
                    let chunk_id = ChunkId(checkpoint.chunk_id as usize);
                    chunks.push(chunk_id);
                    if segment.chunks.is_empty() {
                        segment.first_chunk_id = Some(chunk_id);
                    }

                    segment.chunks.insert(
                        chunk_id,
                        MemModuleCheckPoint {
                            from_addr: checkpoint.from_addr >> 3,
                            from_skip: checkpoint.from_skip,
                            to_addr: checkpoint.to_addr >> 3,
                            to_count: checkpoint.to_count,
                            count: checkpoint.count,
                        },
                    );
                }

                // Collect paged-dense offsets for this segment.
                let mut offsets_base_addr: u32 = 0;
                let mut addr_range_slots: u32 = 0;
                let mut num_pages: u32 = 0;
                let mut present_count: u32 = 0;
                let mut page_single_ptr: *const u32 = std::ptr::null();
                let mut pages_dense_ptr: *const u32 = std::ptr::null();
                let page_starts_ptr = unsafe {
                    bindings::get_mem_segment_offset_pages(
                        self.inner,
                        mem_id as u32,
                        segment_id,
                        &mut offsets_base_addr as *mut u32,
                        &mut addr_range_slots as *mut u32,
                        &mut num_pages as *mut u32,
                        &mut present_count as *mut u32,
                        &mut page_single_ptr as *mut *const u32,
                        &mut pages_dense_ptr as *mut *const u32,
                    )
                };
                assert!(offsets_base_addr > 0);
                if !page_starts_ptr.is_null() && num_pages > 0 {
                    segment.offsets_base_addr = offsets_base_addr;
                    segment.addr_range_slots = addr_range_slots;
                    segment.num_pages = num_pages;
                    segment.present_count = present_count;
                    segment.page_starts =
                        unsafe { std::slice::from_raw_parts(page_starts_ptr, num_pages as usize) }
                            .to_vec();
                    segment.page_single_value =
                        unsafe { std::slice::from_raw_parts(page_single_ptr, num_pages as usize) }
                            .to_vec();
                    let dense_len = present_count as usize * MEM_OFFSETS_PAGE_SIZE as usize;
                    segment.pages_dense = if dense_len == 0 {
                        Vec::new()
                    } else {
                        unsafe { std::slice::from_raw_parts(pages_dense_ptr, dense_len) }.to_vec()
                    };
                }
                plans.push(Plan::new(
                    ZISK_AIRGROUP_ID,
                    *air_id,
                    Some(SegmentId(segment_id as usize)),
                    InstanceType::Instance,
                    CheckPoint::Multiple(chunks),
                    Some(Box::new(segment)),
                ));
            }
        }

        #[cfg(feature = "save_mem_plans")]
        save_plans(&plans, "asm_plans.txt");

        timer_stop_and_log_info!(COLLECT_MEM_PLANS);
        plans
    }

    // Collects memory statistics from the planner.
    // pub fn get_mem_stats(&self) -> Vec<ExecutorStatsEnum> {
    //     let mem_stats_len = unsafe { bindings::get_mem_stats_len(self.inner) };
    //     let mem_stats_ptr = unsafe { bindings::get_mem_stats_ptr(self.inner) as *const u64 };
    //     assert!(
    //         !mem_stats_ptr.is_null() || (mem_stats_len == 0),
    //         "Failed to get memory stats pointer"
    //     );

    //     let mut mem_stats = Vec::with_capacity(mem_stats_len as usize / 4);

    //     let mut i: usize = 0;
    //     for _ in 0..mem_stats_len / 4 {
    //         let id: u64 = unsafe { mem_stats_ptr.add(i).read() };
    //         let start_seconds: u64 = unsafe { mem_stats_ptr.add(i + 1).read() };
    //         let start_nanos: u64 = unsafe { mem_stats_ptr.add(i + 2).read() };
    //         let duration_nanos: u64 = unsafe { mem_stats_ptr.add(i + 3).read() };
    //         i += 4;
    //         let now_system_time = SystemTime::now();
    //         let start_system_time = UNIX_EPOCH + Duration::new(start_seconds, start_nanos as u32);
    //         let difference_system_time =
    //             now_system_time.duration_since(start_system_time).unwrap_or(Duration::ZERO);
    //         let start_time = Instant::now() - difference_system_time;
    //         let duration = Duration::from_nanos(duration_nanos);
    //         match id {
    //             1 => mem_stats.push(ExecutorStatsEnum::MemOpsCountPhase(ExecutorStatsDuration {
    //                 start_time,
    //                 duration,
    //             })),
    //             2 => mem_stats.push(ExecutorStatsEnum::MemOpsPlanPhase(ExecutorStatsDuration {
    //                 start_time,
    //                 duration,
    //             })),
    //             3 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk0(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             4 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk1(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             5 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk2(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             6 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk3(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             7 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk4(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             8 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk5(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             9 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk6(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             10 => {
    //                 mem_stats.push(ExecutorStatsEnum::MemOpsExecuteChunk7(ExecutorStatsDuration {
    //                     start_time,
    //                     duration,
    //                 }))
    //             }
    //             _ => {
    //                 panic!("Unknown memory stats ID: {id}");
    //             }
    //         }
    //     }

    //     mem_stats
    // }
}

impl Drop for MemPlanner {
    fn drop(&mut self) {
        unsafe {
            bindings::destroy_mem_count_and_plan(self.inner);
        }
    }
}
