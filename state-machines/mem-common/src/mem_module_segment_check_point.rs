use std::collections::HashMap;

use crate::MemModuleCheckPoint;
use zisk_common::ChunkId;

#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub chunks: HashMap<ChunkId, MemModuleCheckPoint>,
    pub first_chunk_id: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl MemModuleSegmentCheckPoint {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { chunks: HashMap::new(), first_chunk_id: None, is_last_segment: false }
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
