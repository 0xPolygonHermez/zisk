//! Turning the first segment of the three memory areas into one `CompactMem` plan.
//!
//! Both planners -- the Rust one and the wrapper over the C++ one -- segment each memory area
//! exactly as they did before this air existed: the blocks of `CompactMem` are sized like the airs
//! they take the segment from, so a segment planned for `Mem` fits the `mem_` block unchanged. What
//! changes is only the assembly: instead of three plans for the three segment 0s, one plan for the
//! fused air carrying the three checkpoints.
//!
//! Every execution reads program data and has RAM traffic, and the free-input area is read whenever
//! the program takes an input, so in practice the three are always there and the fusion always
//! happens -- saving two instances per execution, which is what the recursion is paid by. When one
//! of them is missing (an execution with no free input, say), there is nothing to fuse and the
//! plans are left exactly as they were.

use zisk_common::{CheckPoint, ChunkId, InstanceType, Plan, SegmentId};
use zisk_pil::{COMPACT_MEM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{CompactMemSegmentCheckPoint, MemModuleSegmentCheckPoint};

/// Replaces segment 0 of each area with a single `CompactMem` plan, and appends the rest unchanged.
///
/// `mem`, `input_data` and `rom_data` are that area's plans in segment order, as its own planner
/// produced them. The returned plans are the ones to prove: the fused instance first, then every
/// remaining segment under its own air, keeping its segment id -- the continuation chain of an area
/// is walked by `CompactMem` for segment 0 and by the standalone air from segment 1 on.
pub fn fuse_first_segments(
    mut mem: Vec<Plan>,
    mut input_data: Vec<Plan>,
    mut rom_data: Vec<Plan>,
) -> Vec<Plan> {
    if mem.is_empty() || input_data.is_empty() || rom_data.is_empty() {
        let mut plans = mem;
        plans.append(&mut input_data);
        plans.append(&mut rom_data);
        return plans;
    }

    let mem_first = mem.remove(0);
    let input_first = input_data.remove(0);
    let rom_first = rom_data.remove(0);

    let mut plans = vec![compact_plan(mem_first, input_first, rom_first)];
    plans.append(&mut mem);
    plans.append(&mut input_data);
    plans.append(&mut rom_data);
    plans
}

/// The fused plan: the three checkpoints, the union of the chunks the three subscribe to, and the
/// occupancy of the three added up (the instance is as full as the sum of its blocks).
fn compact_plan(mem: Plan, input_data: Plan, rom_data: Plan) -> Plan {
    debug_assert_eq!(mem.segment_id, Some(SegmentId(0)));
    debug_assert_eq!(input_data.segment_id, Some(SegmentId(0)));
    debug_assert_eq!(rom_data.segment_id, Some(SegmentId(0)));

    // Only when the three planners report it: the sum of two known figures and one missing one
    // would read as a low occupancy rather than as an unknown.
    let occupancy = [&mem, &input_data, &rom_data]
        .iter()
        .map(|plan| plan.occupancy.as_ref().map(|o| (o.used, o.capacity)))
        .try_fold((0u64, 0u64), |(used, capacity), block| {
            block.map(|(u, c)| (used + u, capacity + c))
        });

    let chunks =
        union_of_chunks(&[&mem.check_point, &input_data.check_point, &rom_data.check_point]);

    let check_point = CompactMemSegmentCheckPoint::new(
        take_segment_check_point(mem),
        take_segment_check_point(input_data),
        take_segment_check_point(rom_data),
    );

    let plan = Plan::new(
        ZISK_AIRGROUP_ID,
        COMPACT_MEM_AIR_IDS[0],
        Some(SegmentId(0)),
        InstanceType::Instance,
        CheckPoint::Multiple(chunks),
        Some(Box::new(check_point)),
    );

    match occupancy {
        Some((used, capacity)) => plan.with_occupancy(used, capacity),
        None => plan,
    }
}

/// The chunks the fused instance has to be fed, which is every chunk any of its blocks needs. Kept
/// sorted and deduplicated: the collect phase indexes an instance's collectors by the position of
/// the chunk in this list.
fn union_of_chunks(check_points: &[&CheckPoint]) -> Vec<ChunkId> {
    let mut chunks: Vec<ChunkId> = check_points
        .iter()
        .flat_map(|check_point| match check_point {
            CheckPoint::None => Vec::new(),
            CheckPoint::Single(chunk_id) => vec![*chunk_id],
            CheckPoint::Multiple(chunk_ids) => chunk_ids.clone(),
        })
        .collect();
    chunks.sort_unstable();
    chunks.dedup();
    chunks
}

fn take_segment_check_point(plan: Plan) -> MemModuleSegmentCheckPoint {
    *plan
        .meta
        .expect("a memory plan always carries its segment checkpoint")
        .downcast::<MemModuleSegmentCheckPoint>()
        .expect("a memory plan's meta is a MemModuleSegmentCheckPoint")
}
