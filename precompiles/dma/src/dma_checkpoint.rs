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
    /// Inputs the instance's collectors hold over every chunk: each one reserves its chunk's.
    pub fn inputs(&self) -> u64 {
        self.chunks.values().map(|(inputs, _)| inputs).sum()
    }

    /// Rows the instance collects over every chunk.
    pub fn rows(&self) -> u64 {
        self.chunks.values().map(|(_, counters)| counters.rows()).sum()
    }

    /// The plan of an instance, from its `meta`.
    pub fn of(ictx: &zisk_common::InstanceCtx) -> &Self {
        ictx.plan.meta.as_ref().unwrap().downcast_ref::<Self>().unwrap()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_common::CollectCounter;

    fn counters([memcpy, inputcpy, memset, memcmp]: [u32; 4]) -> DmaCollectCounters {
        DmaCollectCounters {
            memcpy: CollectCounter::new(0, memcpy),
            inputcpy: CollectCounter::new(0, inputcpy),
            memset: CollectCounter::new(0, memset),
            memcmp: CollectCounter::new(0, memcmp),
        }
    }

    #[test]
    fn an_instance_collects_the_inputs_and_rows_of_every_chunk() {
        let checkpoint = DmaCheckPoint {
            chunks: HashMap::from([
                (ChunkId(1), (3, counters([10, 0, 5, 0]))),
                (ChunkId(2), (2, counters([0, 7, 0, 1]))),
            ]),
            last_chunk: Some(ChunkId(2)),
            is_last_segment: false,
        };
        assert_eq!(checkpoint.inputs(), 5);
        assert_eq!(checkpoint.rows(), 23);
        assert_eq!(DmaCheckPoint::default().rows(), 0);
    }
}
