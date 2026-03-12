use std::collections::HashMap;

use zisk_common::ChunkId;

use crate::DmaCollectCounters;

#[derive(Default, Debug)]
pub struct DmaCheckPoint {
    pub chunks: HashMap<ChunkId, (u64, DmaCollectCounters)>,
    pub last_chunk: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl DmaCheckPoint {
    #[cfg(any(feature = "save_dma_collectors", feature = "save_dma_plans"))]
    pub fn get_debug_info(&self, title: &str, segment_id: u64) -> String {
        self.chunks
            .iter()
            .map(|(chunk_id, (num_inputs, collect_counters))| {
                format!(
                    "{title} #{segment_id}@{chunk_id} [{num_inputs}|{}]{}{}",
                    collect_counters.get_debug_info(),
                    if Some(*chunk_id) == self.last_chunk { " [last_chunk]" } else { "" },
                    if self.is_last_segment { " [last_segment]" } else { "" },
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
