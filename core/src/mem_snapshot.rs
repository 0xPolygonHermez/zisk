//! Snapshots of memory regions taken at a given *temporal reference*.
//!
//! The `mt` family of DMA operations (`dma_mtcpy`, `dma_mtcmp` and their extended variants) read
//! their source not from the current memory contents but from the contents the source had at an
//! earlier point of the execution, identified by a temporal reference (a `step` value obtained by
//! the guest with the `flag` operation).
//!
//! Keeping a full memory history would be prohibitively expensive, so the guest has to announce in
//! advance which region it will want to read back, with the `execute_advice` operation.  That
//! operation copies the region here, tagged with the temporal reference that was last requested,
//! and the `mt` operations then serve their source from this store.
//!
//! Only the [`MEM_SNAPSHOT_GENERATIONS`] most recently created temporal references are kept alive;
//! older ones are evicted so the store stays bounded no matter how long the execution runs.

use std::collections::{HashMap, VecDeque};

/// Number of distinct temporal references whose regions are kept alive at any time.
///
/// Sized for a long-range memo rather than for short-range buffer reuse: the guest's Keccak-f
/// cache files one temporal reference for the input and one for the output of every distinct
/// permutation, and a hit can land on any of them however long ago it was created, so a small
/// window makes the memo useless (its own geometry table shows reuse reaching tens of thousands
/// of entries back).  This is only a cap: a generation is created when something is captured
/// under it, so the store costs nothing until it is used, and a block that runs 218k
/// permutations holds ~87 MB of snapshots -- outside the proof, where it buys 2x200 bytes of
/// *proven* copying per permutation.
pub const MEM_SNAPSHOT_GENERATIONS: usize = 1 << 20;

/// One contiguous chunk of memory captured by `execute_advice`.
///
/// `addr` is always 8-byte aligned and `data.len()` is always a multiple of 8: the requested range
/// is widened outwards to its 64-bit envelope so that the `mt` operations, which read their source
/// as 64-bit words, always find whole words in the snapshot.
#[derive(Debug, Clone)]
pub struct MemSnapshotRegion {
    pub addr: u64,
    pub data: Vec<u8>,
}

impl MemSnapshotRegion {
    /// Address just past the last captured byte
    #[inline(always)]
    pub fn end(&self) -> u64 {
        self.addr + self.data.len() as u64
    }

    /// Whether `[addr, addr + count)` falls entirely inside this region
    #[inline(always)]
    pub fn contains(&self, addr: u64, count: u64) -> bool {
        addr >= self.addr && (addr + count) <= self.end()
    }
}

/// Bounded store of memory snapshots, indexed by temporal reference
///
/// Lookup is by hash rather than by scanning: with [`MEM_SNAPSHOT_GENERATIONS`] in the millions a
/// linear scan would dominate emulation time, and both `capture` and `read` run once per
/// permutation.
#[derive(Debug, Clone, Default)]
pub struct MemSnapshots {
    /// Regions of each live temporal reference
    by_ref: HashMap<u64, Vec<MemSnapshotRegion>>,
    /// Live temporal references in creation order, oldest first, for eviction
    order: VecDeque<u64>,
}

impl MemSnapshots {
    pub fn new() -> Self {
        Self { by_ref: HashMap::new(), order: VecDeque::new() }
    }

    /// Adds a region to the generation of `temporal_ref`, creating the generation (and evicting the
    /// oldest one if the store is full) when it is the first region of that temporal reference.
    ///
    /// `addr` and `data` must already be the 64-bit envelope of the advised range.
    pub fn capture(&mut self, temporal_ref: u64, addr: u64, data: Vec<u8>) {
        debug_assert_eq!(addr & 0x07, 0);
        debug_assert_eq!(data.len() & 0x07, 0);

        if let Some(regions) = self.by_ref.get_mut(&temporal_ref) {
            // Replace a region that covers exactly the same range instead of piling up duplicates,
            // which is what a loop that re-advises the same buffer every iteration would do.
            if let Some(region) =
                regions.iter_mut().find(|r| r.addr == addr && r.data.len() == data.len())
            {
                region.data = data;
            } else {
                regions.push(MemSnapshotRegion { addr, data });
            }
            return;
        }

        if self.order.len() == MEM_SNAPSHOT_GENERATIONS {
            if let Some(evicted) = self.order.pop_front() {
                self.by_ref.remove(&evicted);
            }
        }
        self.by_ref.insert(temporal_ref, vec![MemSnapshotRegion { addr, data }]);
        self.order.push_back(temporal_ref);
    }

    /// Returns the captured bytes of `[addr, addr + count)` as of `temporal_ref`, or `None` if no
    /// live generation of that temporal reference covers the whole range.
    pub fn read(&self, temporal_ref: u64, addr: u64, count: u64) -> Option<&[u8]> {
        if count == 0 {
            return Some(&[]);
        }
        let regions = self.by_ref.get(&temporal_ref)?;
        let region = regions.iter().find(|r| r.contains(addr, count))?;
        let offset = (addr - region.addr) as usize;
        Some(&region.data[offset..offset + count as usize])
    }

    /// Human-readable description of what is available for `temporal_ref`, for panic messages
    pub fn describe(&self, temporal_ref: u64) -> String {
        match self.by_ref.get(&temporal_ref) {
            Some(regions) => format!(
                "temporal reference {temporal_ref} has regions [{}]",
                regions
                    .iter()
                    .map(|r| format!("0x{:08X}..0x{:08X}", r.addr, r.end()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            None => format!(
                "temporal reference {temporal_ref} has no live snapshot; {} live references, \
                 oldest {:?}, newest {:?}",
                self.order.len(),
                self.order.front(),
                self.order.back()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_returns_the_captured_bytes() {
        let mut snapshots = MemSnapshots::new();
        snapshots.capture(7, 0x100, (0u8..16).collect());

        assert_eq!(snapshots.read(7, 0x100, 4), Some(&[0u8, 1, 2, 3][..]));
        assert_eq!(snapshots.read(7, 0x105, 3), Some(&[5u8, 6, 7][..]));
        assert_eq!(snapshots.read(7, 0x100, 16).map(|d| d.len()), Some(16));
    }

    #[test]
    fn read_rejects_ranges_that_are_not_fully_covered() {
        let mut snapshots = MemSnapshots::new();
        snapshots.capture(7, 0x100, vec![0; 16]);

        assert!(snapshots.read(7, 0x100, 17).is_none());
        assert!(snapshots.read(7, 0x0F8, 8).is_none());
        assert!(snapshots.read(8, 0x100, 8).is_none());
    }

    #[test]
    fn a_temporal_reference_can_hold_several_regions() {
        let mut snapshots = MemSnapshots::new();
        snapshots.capture(7, 0x100, vec![1; 8]);
        snapshots.capture(7, 0x200, vec![2; 8]);

        assert_eq!(snapshots.read(7, 0x100, 1), Some(&[1u8][..]));
        assert_eq!(snapshots.read(7, 0x200, 1), Some(&[2u8][..]));
    }

    #[test]
    fn recapturing_the_same_range_replaces_it() {
        let mut snapshots = MemSnapshots::new();
        snapshots.capture(7, 0x100, vec![1; 8]);
        snapshots.capture(7, 0x100, vec![2; 8]);

        assert_eq!(snapshots.read(7, 0x100, 1), Some(&[2u8][..]));
    }

    #[test]
    fn the_oldest_generation_is_evicted_when_the_store_is_full() {
        let mut snapshots = MemSnapshots::new();
        for tref in 0..(MEM_SNAPSHOT_GENERATIONS as u64 + 1) {
            snapshots.capture(tref, 0x100, vec![tref as u8; 8]);
        }

        assert!(snapshots.read(0, 0x100, 8).is_none());
        assert!(snapshots.read(1, 0x100, 8).is_some());
        assert!(snapshots.read(MEM_SNAPSHOT_GENERATIONS as u64, 0x100, 8).is_some());
    }
}
