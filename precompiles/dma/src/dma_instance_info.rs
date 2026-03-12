use std::collections::HashMap;

use zisk_common::ChunkId;

use crate::DmaCollectCounters;

#[derive(Debug)]
pub struct DmaInstanceInfo {
    pub chunks: HashMap<ChunkId, (u64, DmaCollectCounters)>,
    pub last_chunk: Option<ChunkId>,
}
