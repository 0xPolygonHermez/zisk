use std::{collections::HashMap, os::raw::c_void};

use crate::*;
use mem_common::{MemAlignCheckPoint, MemModuleCheckPoint, MemModuleSegmentCheckPoint};
use zisk_common::{CheckPoint, ChunkId, InstanceType, Plan, SegmentId};
use zisk_pil::{INPUT_DATA_AIR_IDS, MEM_AIR_IDS, MEM_ALIGN_AIR_IDS, ROM_AIR_IDS, ZISK_AIRGROUP_ID};

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

    /// Retrieves a Vec of memory plans.
    ///
    /// # Safety
    /// This function assumes the underlying C++ memory is valid and the pointer returned
    /// is safe to read for `count` elements.
    pub fn mem_segments(&self) -> Vec<Plan> {
        let mut plans: Vec<Plan> = Vec::new();

        for (mem_id, air_id) in
            [ROM_AIR_IDS[0], INPUT_DATA_AIR_IDS[0], MEM_AIR_IDS[0]].iter().enumerate()
        {
            let mem_segments_count: u32 =
                unsafe { bindings::get_mem_segment_count(self.inner, mem_id as u32) };
            for segment_id in 0..mem_segments_count {
                let mut chunks: Vec<ChunkId> = Vec::new();
                let mut segment = MemModuleSegmentCheckPoint::new();
                let checkpoints = CppMemCheckPoint::from_cpp(self, mem_id as u32, segment_id);
                for checkpoint in checkpoints {
                    let chunk_id = ChunkId(checkpoint.chunk_id as usize);
                    chunks.push(chunk_id);
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

        let mem_align_check_points = CppMemAlignCheckPoint::from_cpp(self);
        let mut last_segment_id = None;
        let mut segment: HashMap<ChunkId, MemAlignCheckPoint> = HashMap::new();
        let mut chunks: Vec<ChunkId> = Vec::new();
        for checkpoint in mem_align_check_points {
            let current_segment_id = SegmentId(checkpoint.segment_id as usize);
            if Some(current_segment_id) != last_segment_id {
                if last_segment_id.is_some() {
                    // If we have a previous segment, push it to plans
                    plans.push(Plan::new(
                        ZISK_AIRGROUP_ID,
                        MEM_ALIGN_AIR_IDS[0],
                        last_segment_id,
                        InstanceType::Instance,
                        CheckPoint::Multiple(std::mem::take(&mut chunks)),
                        Some(Box::new(std::mem::take(&mut segment))),
                    ));
                }
                last_segment_id = Some(current_segment_id);
            }
            segment.insert(
                ChunkId(checkpoint.chunk_id as usize),
                MemAlignCheckPoint {
                    skip: checkpoint.skip,
                    count: checkpoint.count,
                    rows: checkpoint.rows,
                    offset: checkpoint.offset,
                },
            );
        }
        if !chunks.is_empty() {
            plans.push(Plan::new(
                ZISK_AIRGROUP_ID,
                MEM_ALIGN_AIR_IDS[0],
                Some(last_segment_id.unwrap()),
                InstanceType::Instance,
                CheckPoint::Multiple(std::mem::take(&mut chunks)),
                Some(Box::new(std::mem::take(&mut segment))),
            ));
        }

        plans
    }
}

impl Drop for MemPlanner {
    fn drop(&mut self) {
        unsafe {
            bindings::destroy_mem_count_and_plan(self.inner);
        }
    }
}
