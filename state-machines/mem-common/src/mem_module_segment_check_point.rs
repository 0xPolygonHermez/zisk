use std::collections::HashMap;

use crate::MemModuleCheckPoint;
use zisk_common::ChunkId;

/// Page size for the paged-dense offsets layout.
/// The C++ side mirrors these as
// `MEM_OFFSETS_PAGE_SIZE` and `MEM_OFFSETS_PAGE_ABSENT` in
// `mem-cpp/cpp/instance_meta.hpp`. If you change a value here, change it
// there too — there is no compile-time cross-language check today.
pub const MEM_OFFSETS_PAGE_SIZE_LOG2: u32 = 10;
pub const MEM_OFFSETS_PAGE_SIZE: u32 = 1 << MEM_OFFSETS_PAGE_SIZE_LOG2;
const MEM_OFFSETS_PAGE_MASK: u32 = MEM_OFFSETS_PAGE_SIZE - 1;

/// Sentinel stored in `page_starts[p]` to mark page `p` as absent
/// (i.e. all slots in the page equal `page_single_value[p]`). also defined in 
// `mem-cpp/cpp/instance_meta.hpp`. If you change a value here, change it
// there too — there is no compile-time cross-language check today.
pub const MEM_OFFSETS_PAGE_ABSENT: u32 = u32::MAX;

/// Mirrors struct defined in instance_meta.hpp
#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub chunks: HashMap<ChunkId, MemModuleCheckPoint>,
    pub offsets_base_addr: u32,
    pub addr_range_slots: u32,
    pub num_pages: u32,
    pub present_count: u32,
    pub page_starts: Vec<u32>,
    pub page_single_value: Vec<u32>,
    pub pages_dense: Vec<u32>,
    pub first_chunk_id: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl MemModuleSegmentCheckPoint {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Materialised offset value at qword slot `k` (k < `addr_range_slots`).
    #[inline]
    pub fn offset_at(&self, k: u32) -> u32 {
        debug_assert!(k < self.addr_range_slots);
        let page = (k >> MEM_OFFSETS_PAGE_SIZE_LOG2) as usize;
        let in_page = (k & MEM_OFFSETS_PAGE_MASK) as usize;
        let pidx = self.page_starts[page];
        if pidx == MEM_OFFSETS_PAGE_ABSENT {
            self.page_single_value[page]
        } else {
            self.pages_dense[((pidx as usize) << MEM_OFFSETS_PAGE_SIZE_LOG2) + in_page]
        }
    }

    /// Qword-address of the largest slot `j < k` whose offset value differs
    /// from `offset_at(k)`, or `None` when no such `j` exists.
    #[inline]
    pub fn previous_change_addr_w(&self, k: u32) -> Option<u64> {
        debug_assert!(k < self.addr_range_slots);
        let cur_val = self.offset_at(k);
        let base_w = (self.offsets_base_addr as u64) >> 3;
        let mut page = k >> MEM_OFFSETS_PAGE_SIZE_LOG2;
        let mut upper = (k & MEM_OFFSETS_PAGE_MASK) as usize;
        loop {
            let pidx = self.page_starts[page as usize];
            if pidx == MEM_OFFSETS_PAGE_ABSENT {
                if self.page_single_value[page as usize] != cur_val {
                    return Some(
                        base_w
                            + (page << MEM_OFFSETS_PAGE_SIZE_LOG2) as u64
                            + (MEM_OFFSETS_PAGE_SIZE - 1) as u64,
                    );
                }
            } else {
                let start = (pidx as usize) << MEM_OFFSETS_PAGE_SIZE_LOG2;
                let dense = &self.pages_dense[start..start + MEM_OFFSETS_PAGE_SIZE as usize];
                for j in (0..upper).rev() {
                    if dense[j] != cur_val {
                        return Some(base_w + (page << MEM_OFFSETS_PAGE_SIZE_LOG2) as u64 + j as u64);
                    }
                }
            }
            if page == 0 {
                return None;
            }
            page -= 1;
            upper = MEM_OFFSETS_PAGE_SIZE as usize;
        }
    }

    pub fn to_string(&self, segment_id: usize) -> String {
        let mut result = String::new();
        for (chunk_id, checkpoint) in &self.chunks {
            result = result
                + &format!(
                    "MEM #{}@{} [0x{:08X} s:{}] [0x{:08X} C:{}] C:{}{}{}\n",
                    segment_id,
                    chunk_id,
                    checkpoint.from_addr * 8,
                    checkpoint.from_skip,
                    checkpoint.to_addr * 8,
                    checkpoint.to_count,
                    checkpoint.count,
                    if Some(*chunk_id) == self.first_chunk_id { " [first_chunk]" } else { "" },
                    if self.is_last_segment { " [last_segment]" } else { "" }
                );
        }
        result
    }
}