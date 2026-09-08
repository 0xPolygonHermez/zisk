//! Splitting a `Mem` segment's work into ranges one thread each can fill without touching another
//! thread's slots.
//!
//! # The split is over the offsets table, not over the operations
//!
//! `mem_ops` arrives **unsorted**: the offsets table exists precisely so the fill can place each
//! operation straight into its virtual row in one random-access pass, and the only code that sorts
//! the inputs (`MemModuleInstance::prepare_inputs`) is gated behind `legacy_mem_count_and_plan`. So
//! a contiguous slice of `mem_ops` spans arbitrary addresses and cannot be a thread's share.
//!
//! What is ordered is the offsets table: `offset_at` is a monotone running pointer over address
//! index, so cutting *it* into `k` parts of roughly equal slot count gives each range a contiguous
//! band of addresses and, with it, a contiguous band of slots. Each range then walks the whole
//! operation list and fills only the operations whose address falls in its band.
//!
//! # Why the granularity is a whole address
//!
//! For each operation the fill decides which slot it lands in from `current_offsets[addr_index]`:
//! the first operation of an address takes the slot the offsets table names for it, and every later
//! one continues from where that address left off. `addr_changes` — the flag that picks between
//! those two paths — is precisely "this fill has not seen this address yet".
//!
//! That is what forces the granularity. Were a range to own half of an address's operations, its
//! first one would read `current_offsets == 0`, take the `addr_changes` path, and rewind to the
//! address's *first* slot, overwriting what the other range wrote. Cutting the offsets table gives
//! whole addresses for free.
//!
//! Whole addresses also make every backward read the fill performs (`islot - 1`, on the
//! `!addr_changes` paths) land on a slot the same range wrote, so no range ever reads another's
//! slots and no step or dual state crosses a boundary.
//!
//! Whole addresses are enough because the offsets table already accounts for the dual expansion: it
//! was built by the planner under the same rules the fill applies, so the slots an address consumes
//! are fixed before the fill starts, and range `i` writes exactly `[slot_from_i, slot_from_{i+1})`.

use zisk_sm_mem_common::MemModuleSegmentCheckPoint;

/// One thread's share of a segment: a band of addresses and the slots they occupy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemFillRange {
    /// First address index (into the segment's offsets table) this range owns.
    pub addr_from: u32,
    /// One past the last address index it owns. Addresses in the band with no operation of their
    /// own are included and simply consume no slot.
    pub addr_to: u32,
    /// First slot this range writes: where `addr_from`'s operations begin.
    pub slot_from: usize,
    /// One past the last slot it may write — the next range's `slot_from`, or the segment's slot
    /// count for the last range. Nothing outside `[slot_from, slot_to)` is ever written by it.
    pub slot_to: usize,
}

impl MemFillRange {
    /// Slots this range may write. What the balancing equalises.
    pub fn slots_len_any(&self) -> usize {
        self.slot_to - self.slot_from
    }

    /// Same, for the tests that assert the tiling.
    #[cfg(test)]
    pub fn slots_len(&self) -> usize {
        self.slots_len_any()
    }
}

/// Slot an address's operations start at. `offset_at` is 1-based, and 0 marks the halo address
/// (which belongs to the previous segment and starts this one at slot 0).
#[inline]
fn slot_of(seg: &MemModuleSegmentCheckPoint, addr_index: u32) -> usize {
    seg.offset_at(addr_index).saturating_sub(1) as usize
}

/// Cut a segment's offsets table into at most `k` ranges of roughly equal slot count.
///
/// Returns the ranges in slot order, with no gap and no overlap: range `i`'s `slot_to` is range
/// `i+1`'s `slot_from`, the first range starts at slot 0 and address 0, and the last range's
/// `slot_to` is `num_slots` and its `addr_to` the end of the offsets table. Fewer than `k` ranges
/// come back when the table has no distinct enough cut points, and a segment with no addresses
/// yields none.
pub(crate) fn split_mem_slots(
    seg: &MemModuleSegmentCheckPoint,
    num_slots: usize,
    k: usize,
) -> Vec<MemFillRange> {
    let n_addrs = seg.addr_range_slots;
    if n_addrs == 0 || k == 0 {
        return Vec::new();
    }

    // Balance over the slots the addresses actually use, not over the instance's capacity: a
    // mostly-empty instance has its operations packed at the front, and cutting against `num_slots`
    // would put every cut past the end and hand the whole thing to one range.
    let used_slots = (slot_of(seg, n_addrs - 1) + 1).min(num_slots);

    // Address indices to cut at. `slot_of` is monotone non-decreasing over the table, so each cut
    // is a binary search rather than a scan of what can be millions of addresses.
    let mut cuts: Vec<u32> = Vec::with_capacity(k + 1);
    cuts.push(0);
    let mut last_slot = 0usize;
    for i in 1..k {
        let target = i * used_slots / k;
        // First address whose slots start at or after `target`.
        let mut lo = 0u32;
        let mut hi = n_addrs;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if slot_of(seg, mid) < target {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        // A cut is only worth taking if it opens a non-empty slot band, and two things can make it
        // empty. A run of addresses with no operations shares one offset value, so a cut inside
        // such a run would carve a band of zero slots. And the offsets of the addresses past the
        // last operation carry the running pointer's final value, which is one *past* the last used
        // slot, so a cut there would start a band at `num_slots` and end it there too. Refusing both
        // keeps the addresses and the slots tiling, with no empty range to drop afterwards.
        if lo < n_addrs
            && lo > *cuts.last().unwrap()
            && slot_of(seg, lo) > last_slot
            && slot_of(seg, lo) < num_slots
        {
            last_slot = slot_of(seg, lo);
            cuts.push(lo);
        }
    }
    cuts.push(n_addrs);

    let mut ranges: Vec<MemFillRange> = cuts
        .windows(2)
        .map(|w| MemFillRange {
            addr_from: w[0],
            addr_to: w[1],
            slot_from: if w[0] == 0 { 0 } else { slot_of(seg, w[0]) },
            // Patched below: a range ends where the next one begins.
            slot_to: 0,
        })
        .collect();

    for i in 0..ranges.len() - 1 {
        ranges[i].slot_to = ranges[i + 1].slot_from;
    }
    if let Some(last) = ranges.last_mut() {
        last.slot_to = num_slots;
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a segment whose addresses each carry `slots_per_addr` slots. Mirrors the planner:
    /// `add_addr_offset` takes the 1-based slot of the address's first operation, and the offsets
    /// are a monotone running pointer.
    fn seg_of(base_addr_w: u32, n_addrs: u32, slots_per_addr: u32) -> MemModuleSegmentCheckPoint {
        let mut seg = MemModuleSegmentCheckPoint::default();
        for a in 0..n_addrs {
            seg.add_addr_offset(base_addr_w + a, a * slots_per_addr + 1);
        }
        seg
    }

    /// The property the whole scheme rests on: the ranges tile the addresses and the slots, with no
    /// overlap and no gap. An overlap would have two threads writing the same slot; a gap would
    /// leave a slot unwritten.
    #[test]
    fn ranges_tile_the_addresses_and_the_slots() {
        let (n_addrs, per_addr) = (64u32, 3u32);
        let seg = seg_of(0x1000, n_addrs, per_addr);
        let num_slots = (n_addrs * per_addr) as usize;

        for k in [1usize, 2, 3, 4, 5, 7, 8, 16] {
            let ranges = split_mem_slots(&seg, num_slots, k);
            assert!(!ranges.is_empty(), "k={k}");

            assert_eq!(ranges[0].addr_from, 0, "k={k}");
            assert_eq!(ranges[0].slot_from, 0, "k={k}");
            assert_eq!(ranges.last().unwrap().addr_to, n_addrs, "k={k}");
            assert_eq!(ranges.last().unwrap().slot_to, num_slots, "k={k}");

            for w in ranges.windows(2) {
                assert_eq!(w[0].addr_to, w[1].addr_from, "k={k}: addresses must tile");
                assert_eq!(w[0].slot_to, w[1].slot_from, "k={k}: slots must tile");
            }
            assert_eq!(
                ranges.iter().map(|r| r.slots_len()).sum::<usize>(),
                num_slots,
                "k={k}: every slot belongs to exactly one range"
            );
            assert!(ranges.iter().all(|r| r.slots_len() > 0), "k={k}: no empty range");
        }
    }

    /// With plenty of addresses the shares stay within one address's slots of the ideal.
    #[test]
    fn the_shares_are_balanced() {
        let (n_addrs, per_addr) = (512u32, 4u32);
        let seg = seg_of(0x3000, n_addrs, per_addr);
        let num_slots = (n_addrs * per_addr) as usize;

        for k in [2usize, 4, 8] {
            let ranges = split_mem_slots(&seg, num_slots, k);
            assert_eq!(ranges.len(), k, "k={k}: enough addresses to cut into k");
            let ideal = num_slots / k;
            for r in &ranges {
                assert!(
                    r.slots_len().abs_diff(ideal) <= per_addr as usize,
                    "k={k}: a range of {} slots is more than {per_addr} off the {ideal} ideal",
                    r.slots_len()
                );
            }
        }
    }

    /// A single address cannot be cut, however many slots it holds: one range takes it all.
    #[test]
    fn a_single_address_is_never_cut() {
        let seg = seg_of(0x5000, 1, 16);
        let ranges = split_mem_slots(&seg, 16, 8);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].slots_len(), 16);
        assert_eq!(ranges[0].addr_to, 1);
    }

    /// Fewer addresses than threads: fewer ranges, never empty ones.
    #[test]
    fn fewer_addresses_than_threads_yields_fewer_ranges() {
        let seg = seg_of(0x4000, 3, 2);
        let ranges = split_mem_slots(&seg, 6, 8);
        assert!(ranges.len() <= 3);
        assert!(ranges.iter().all(|r| r.slots_len() > 0));
        assert_eq!(ranges.iter().map(|r| r.slots_len()).sum::<usize>(), 6);
    }

    /// Long runs of addresses with no operations share one offset value, so a cut can land inside
    /// such a run. Those ranges hold no slots and must be folded away rather than handed out.
    #[test]
    fn runs_of_empty_addresses_do_not_produce_empty_ranges() {
        let base = 0x7000;
        let mut seg = MemModuleSegmentCheckPoint::default();
        let mut slot = 1;
        for a in 0..64u32 {
            seg.add_addr_offset(base + a, slot);
            // Only every eighth address carries operations.
            if a % 8 == 0 {
                slot += 2;
            }
        }
        let num_slots = 16;
        for k in [2usize, 4, 8, 16] {
            let ranges = split_mem_slots(&seg, num_slots, k);
            assert!(
                ranges.iter().all(|r| r.slots_len() > 0),
                "k={k}: an empty range slipped through"
            );
            for w in ranges.windows(2) {
                assert_eq!(w[0].slot_to, w[1].slot_from, "k={k}: slots must still tile");
                assert_eq!(w[0].addr_to, w[1].addr_from, "k={k}: addresses must still tile");
            }
            assert_eq!(ranges.last().unwrap().slot_to, num_slots, "k={k}");
            assert_eq!(ranges.last().unwrap().addr_to, 64, "k={k}");
        }
    }

    #[test]
    fn no_addresses_yields_no_ranges() {
        let seg = MemModuleSegmentCheckPoint::default();
        assert!(split_mem_slots(&seg, 8, 8).is_empty());
    }
}
