use std::collections::HashMap;

use crate::MemModuleCheckPoint;
use zisk_common::ChunkId;

#[derive(Debug, Default, Clone)]
pub struct MemModuleSegmentCheckPoint {
    pub chunks: HashMap<ChunkId, MemModuleCheckPoint>,
    /// Byte address of the first qword slot in this segment.
    pub offsets_base_addr: u32,
    /// Sparse offset table — change-points only.
    ///
    /// `offset_change_slots[k]` is the slot index (within `[0,
    /// addr_range_slots)`) where the offset value changes. `slots[0]` is
    /// always 0. Adjacent values in `offset_change_values` are guaranteed to
    /// differ (otherwise the runs would have been merged).
    pub offset_change_slots: Vec<u32>,
    pub offset_change_values: Vec<u32>,
    /// Total slot count = (last_addr - first_addr)/8 + 1. Carried through to
    /// the consumer so it can size its dense `current_offsets` working
    /// buffer.
    pub addr_range_slots: u32,
    pub first_chunk_id: Option<ChunkId>,
    pub is_last_segment: bool,
}

impl MemModuleSegmentCheckPoint {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            offsets_base_addr: 0,
            offset_change_slots: Vec::new(),
            offset_change_values: Vec::new(),
            addr_range_slots: 0,
            first_chunk_id: None,
            is_last_segment: false,
        }
    }

    /// Materialised offset value at qword slot `k` (k < `addr_range_slots`).
    /// Cost: one `partition_point` binary search over the sparse slot array.
    /// Assumes the SoA invariant `offset_change_slots[0] == 0`.
    #[inline]
    pub fn offset_at(&self, k: u32) -> u32 {
        debug_assert!(!self.offset_change_slots.is_empty());
        debug_assert_eq!(self.offset_change_slots[0], 0);
        debug_assert!(k < self.addr_range_slots);
        let idx = self.offset_change_slots.partition_point(|&s| s <= k);
        // idx >= 1 because slots[0] == 0 <= k always.
        self.offset_change_values[idx - 1]
    }

    /// Qword-address of the change-point strictly before slot `k`, or `None`
    /// when `k` lies in the first run (no earlier distinct slot). Replaces
    /// the linear backward scan from `mem_module::get_previous_addr_w`.
    #[inline]
    pub fn previous_change_addr_w(&self, k: u32) -> Option<u64> {
        debug_assert!(!self.offset_change_slots.is_empty());
        let idx = self.offset_change_slots.partition_point(|&s| s <= k);
        if idx < 2 {
            return None;
        }
        // The previous run starts at slots[idx - 2]; its last slot is
        // slots[idx - 1] - 1. We return the qword address of slots[idx - 2]
        // (the BEGINNING of the previous run) — semantically the "previous
        // distinct address" that the dense backward scan would have found.
        Some((self.offsets_base_addr as u64 >> 3) + self.offset_change_slots[idx - 2] as u64)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn from_dense(dense: &[u32], base_addr: u32) -> MemModuleSegmentCheckPoint {
        // Build the sparse representation from a dense reference. Used in
        // tests + as the model for the equality validator.
        let mut slots = Vec::new();
        let mut values = Vec::new();
        let mut prev = u32::MAX;
        for (i, &v) in dense.iter().enumerate() {
            if v != prev {
                slots.push(i as u32);
                values.push(v);
                prev = v;
            }
        }
        MemModuleSegmentCheckPoint {
            chunks: HashMap::new(),
            offsets_base_addr: base_addr,
            offset_change_slots: slots,
            offset_change_values: values,
            addr_range_slots: dense.len() as u32,
            first_chunk_id: None,
            is_last_segment: false,
        }
    }

    #[test]
    fn offset_at_matches_dense() {
        let dense: Vec<u32> = vec![0, 1, 1, 1, 2, 3, 3, 5, 5, 5, 7];
        let seg = from_dense(&dense, 0x1000);
        for (i, &v) in dense.iter().enumerate() {
            assert_eq!(seg.offset_at(i as u32), v, "mismatch at index {i}");
        }
    }

    #[test]
    fn offset_at_all_same() {
        let dense: Vec<u32> = vec![5; 100];
        let seg = from_dense(&dense, 0);
        for i in 0..100 {
            assert_eq!(seg.offset_at(i as u32), 5);
        }
        // Single change-point: slot 0 → value 5
        assert_eq!(seg.offset_change_slots, vec![0]);
        assert_eq!(seg.offset_change_values, vec![5]);
    }

    #[test]
    fn offset_at_every_slot_different() {
        let dense: Vec<u32> = (10..30).collect();
        let seg = from_dense(&dense, 0);
        for (i, &v) in dense.iter().enumerate() {
            assert_eq!(seg.offset_at(i as u32), v);
        }
        // Worst case: change-points equal slot count
        assert_eq!(seg.offset_change_slots.len(), dense.len());
    }

    #[test]
    fn previous_change_addr_w_basic() {
        // dense:  [0, 1, 1, 2, 2, 2, 5]
        // slots:  [0, 1, 3, 6]
        // values: [0, 1, 2, 5]
        let dense: Vec<u32> = vec![0, 1, 1, 2, 2, 2, 5];
        let seg = from_dense(&dense, 0x8000); // base_addr in bytes; >> 3 = 0x1000
        // k=0 → in first run, no previous distinct slot
        assert_eq!(seg.previous_change_addr_w(0), None);
        // k=1 → idx=2 → prev change at slot 0 → addr 0x1000
        assert_eq!(seg.previous_change_addr_w(1), Some(0x1000));
        // k=2 → idx=2 → prev change at slot 0 → addr 0x1000
        assert_eq!(seg.previous_change_addr_w(2), Some(0x1000));
        // k=3 → idx=3 → prev change at slot 1 → addr 0x1000 + 1 = 0x1001
        assert_eq!(seg.previous_change_addr_w(3), Some(0x1001));
        // k=6 → idx=4 → prev change at slot 3 → addr 0x1003
        assert_eq!(seg.previous_change_addr_w(6), Some(0x1003));
    }

    #[test]
    fn halo_slot_zero() {
        // First segment of a kind has slot 0 == 0 (halo); subsequent segments
        // have slot 0 == r+1 for some r. Both port through unchanged via
        // `offset_change_values[0]`.
        let dense: Vec<u32> = vec![0, 0, 0, 1, 2, 2];
        let seg = from_dense(&dense, 0);
        assert_eq!(seg.offset_change_values[0], 0);
        assert_eq!(seg.offset_at(0), 0);
        assert_eq!(seg.offset_at(2), 0);
        assert_eq!(seg.offset_at(3), 1);
    }
}
