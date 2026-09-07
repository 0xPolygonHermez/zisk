//! The `Mem` air family: three airs over the same template, differing only in `lanes_x_row`.
//!
//! `Mem`, `MemLarge` and `MemHuge` prove the same constraints and commit the same columns; the
//! only difference is how many memory lanes ride on a row, and therefore how many memory
//! operations one instance holds. That count is what this module calls the air's **slots**.
//!
//! # Why planning always uses the widest air
//!
//! The planner emits, per segment, an offsets table that maps an address to the slot where its
//! first operation lands. Those slots are absolute positions inside the segment, counted from
//! zero, so a table built for a segment of `S` slots stays correct in ANY air with at least as
//! many slots — the trace is simply padded further down. It does NOT survive the other
//! direction: an offset past the end of a narrower air has nowhere to go.
//!
//! So count-and-plan budgets every segment with [`mem_planning_slots`], the widest air's
//! capacity, and the plans come out on that air. Only afterwards, once a segment's real
//! occupancy is known, can it be moved down to the narrowest air that still holds it — which is
//! what [`shrink_last_mem_plan`] does for the segment that has one: the last, the only one that
//! is not full by construction.
//!
//! Nothing here forbids a mixed run: the airs are independent, and a segment proved on `Mem`
//! chains with one proved on `MemHuge` exactly as two `MemHuge` segments would.

use zisk_common::Plan;
use zisk_pil::{
    MemHugeTrace, MemLargeTrace, MemTrace, MEM_AIR_IDS, MEM_HUGE_AIR_IDS, MEM_LARGE_AIR_IDS,
};

use crate::{
    mem_huge_lanes_x_row, mem_lanes_x_row, mem_large_lanes_x_row, MemModuleSegmentCheckPoint,
};

/// Memory lanes each `Mem` air packs on a row, i.e. its `lanes_x_row` in `pil/zisk.pil`.
///
/// These are consts because the witness needs the width at compile time to size a row's lane
/// loop, while [`mem_airs`] reads it off the generated rows at run time. The two are pinned
/// together by [`tests::the_constants_match_the_generated_rows`], so a change in the PIL that is
/// not mirrored here fails that test rather than corrupting a trace.
pub mod lanes_x_row {
    /// `Mem`.
    pub const MEM: usize = 1;
    /// `MemLarge`.
    pub const MEM_LARGE: usize = 4;
    /// `MemHuge`.
    pub const MEM_HUGE: usize = 8;
}

/// One air of the `Mem` family: which air it is, and how much it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemAir {
    /// Air id in the Zisk airgroup.
    pub air_id: usize,
    /// Rows the air commits.
    pub num_rows: usize,
    /// Memory lanes packed on each row.
    pub lanes_x_row: usize,
}

impl MemAir {
    /// Memory operations one instance of this air holds — its rows times its lanes.
    #[inline(always)]
    pub fn slots(&self) -> usize {
        self.num_rows * self.lanes_x_row
    }
}

/// The `Mem` airs ordered from the narrowest to the widest.
///
/// Sizes come from the generated traces and rows, never from constants restated here, so the
/// order and the capacities follow `pil/zisk.pil` on their own.
pub fn mem_airs() -> [MemAir; 3] {
    let mut airs = [
        MemAir {
            air_id: MEM_AIR_IDS[0],
            num_rows: MemTrace::<()>::NUM_ROWS,
            lanes_x_row: mem_lanes_x_row(),
        },
        MemAir {
            air_id: MEM_LARGE_AIR_IDS[0],
            num_rows: MemLargeTrace::<()>::NUM_ROWS,
            lanes_x_row: mem_large_lanes_x_row(),
        },
        MemAir {
            air_id: MEM_HUGE_AIR_IDS[0],
            num_rows: MemHugeTrace::<()>::NUM_ROWS,
            lanes_x_row: mem_huge_lanes_x_row(),
        },
    ];
    // The declaration order in zisk.pil is the intended one, but the planner's correctness rests
    // on "widest last", not on the PIL author's ordering, so sort rather than assume.
    airs.sort_by_key(|air| air.slots());
    airs
}

/// The widest `Mem` air, the one count-and-plan budgets every segment with.
pub fn mem_planning_air() -> MemAir {
    // `mem_airs` is sorted by capacity, so the widest is the last.
    mem_airs()[2]
}

/// Memory operations count-and-plan gives a `Mem` segment: the widest air's capacity.
pub fn mem_planning_slots() -> usize {
    mem_planning_air().slots()
}

/// The narrowest `Mem` air that still holds `slots` operations.
///
/// Falls back to the widest when nothing narrower fits, so a segment planned at
/// [`mem_planning_slots`] always gets an answer.
pub fn mem_air_for_slots(slots: usize) -> MemAir {
    let airs = mem_airs();
    *airs.iter().find(|air| air.slots() >= slots).unwrap_or(&airs[2])
}

/// Moves the last `Mem` segment down to the narrowest air that still holds it.
///
/// Every segment but the last is full by construction — the planner only opens a new one once
/// the current has no rows left — so the last is the only one with room to shrink, and the only
/// one this touches. It rewrites nothing but `plan.air_id`: the segment's offsets are absolute
/// slot positions and stay valid in the narrower air (see the module docs).
///
/// `plans` is the whole plan list; the `Mem` segments in it are identified by their air id and
/// their `is_last_segment` flag, so a list carrying other airs (or no `Mem` at all) is fine.
pub fn shrink_last_mem_plan(plans: &mut [Plan]) {
    let planning_air_id = mem_planning_air().air_id;
    for plan in plans.iter_mut() {
        if plan.air_id != planning_air_id {
            continue;
        }
        let Some(segment) =
            plan.meta.as_ref().and_then(|m| m.downcast_ref::<MemModuleSegmentCheckPoint>())
        else {
            continue;
        };
        if !segment.is_last_segment {
            continue;
        }
        plan.air_id = mem_air_for_slots(segment.used_slots() as usize).air_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemModuleCheckPoint;
    use zisk_common::{CheckPoint, ChunkId, InstanceType, SegmentId};

    /// A `Mem` plan on the planning air holding `used` operations, spread over two chunks so the
    /// per-chunk accounting `used_slots` sums is actually exercised.
    fn mem_plan(segment_id: usize, used: u32, is_last_segment: bool) -> Plan {
        let mut segment = MemModuleSegmentCheckPoint { is_last_segment, ..Default::default() };
        segment.chunks.insert(ChunkId(0), MemModuleCheckPoint::new(0, 0, used / 2));
        segment.chunks.insert(ChunkId(1), MemModuleCheckPoint::new(1, 0, used - used / 2));
        Plan::new(
            0,
            mem_planning_air().air_id,
            Some(SegmentId(segment_id)),
            InstanceType::Instance,
            CheckPoint::Multiple(vec![ChunkId(0), ChunkId(1)]),
            Some(Box::new(segment)),
        )
    }

    #[test]
    fn only_the_last_segment_is_shrunk() {
        let airs = mem_airs();
        let full = airs[2].slots() as u32;
        // A full middle segment and a nearly empty last one.
        let mut plans = vec![mem_plan(0, full, false), mem_plan(1, 2, true)];
        shrink_last_mem_plan(&mut plans);

        assert_eq!(plans[0].air_id, airs[2].air_id, "a full segment stays on the planning air");
        assert_eq!(plans[1].air_id, airs[0].air_id, "the last segment moves to the narrowest air");
    }

    #[test]
    fn a_last_segment_that_needs_the_width_keeps_it() {
        let airs = mem_airs();
        // One operation past the middle air is what forces the widest one.
        let mut plans = vec![mem_plan(0, airs[1].slots() as u32 + 1, true)];
        shrink_last_mem_plan(&mut plans);
        assert_eq!(plans[0].air_id, airs[2].air_id);

        // And exactly the middle air's capacity fits in it.
        let mut plans = vec![mem_plan(0, airs[1].slots() as u32, true)];
        shrink_last_mem_plan(&mut plans);
        assert_eq!(plans[0].air_id, airs[1].air_id);
    }

    #[test]
    fn plans_of_other_airs_are_left_alone() {
        // The C++ path hands the whole plan list over, RomData / InputData / MemAlign included.
        let mut other = mem_plan(0, 2, true);
        other.air_id = mem_planning_air().air_id + 1000;
        let air_id_before = other.air_id;
        let mut plans = vec![other];
        shrink_last_mem_plan(&mut plans);
        assert_eq!(plans[0].air_id, air_id_before);
    }

    /// [`lanes_x_row`] drives the witness and the generated rows drive the air; a disagreement
    /// between them would show up as a trace that does not match its air.
    #[test]
    fn the_constants_match_the_generated_rows() {
        assert_eq!(mem_lanes_x_row(), lanes_x_row::MEM);
        assert_eq!(mem_large_lanes_x_row(), lanes_x_row::MEM_LARGE);
        assert_eq!(mem_huge_lanes_x_row(), lanes_x_row::MEM_HUGE);
    }

    #[test]
    fn the_airs_are_ordered_from_narrowest_to_widest() {
        let airs = mem_airs();
        assert!(airs[0].slots() <= airs[1].slots());
        assert!(airs[1].slots() <= airs[2].slots());
        assert_eq!(mem_planning_slots(), airs[2].slots());
    }

    #[test]
    fn every_air_has_at_least_one_lane() {
        for air in mem_airs() {
            assert!(air.lanes_x_row >= 1, "air {} packs no lane", air.air_id);
            assert!(air.num_rows > 0, "air {} has no row", air.air_id);
        }
    }

    #[test]
    fn a_segment_gets_the_narrowest_air_that_holds_it() {
        let airs = mem_airs();
        // Exactly full is a fit: capacity is inclusive.
        assert_eq!(mem_air_for_slots(airs[0].slots()).air_id, airs[0].air_id);
        // One operation past an air's capacity moves up to the next one.
        assert_eq!(mem_air_for_slots(airs[0].slots() + 1).air_id, airs[1].air_id);
        assert_eq!(mem_air_for_slots(airs[1].slots() + 1).air_id, airs[2].air_id);
        // An empty segment fits anywhere, so it takes the narrowest.
        assert_eq!(mem_air_for_slots(0).air_id, airs[0].air_id);
    }

    #[test]
    fn a_segment_wider_than_every_air_falls_back_to_the_widest() {
        // Not reachable from the planner (it never over-fills a segment), but the caller gets a
        // usable air rather than a panic if it ever were.
        let airs = mem_airs();
        assert_eq!(mem_air_for_slots(airs[2].slots() + 1).air_id, airs[2].air_id);
    }
}
