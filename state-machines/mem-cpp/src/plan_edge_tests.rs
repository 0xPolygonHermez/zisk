//! The GPU planner against the CPU planner on synthetic memory-ops streams, at the edges a block
//! rarely reaches: a region's last word, an empty chunk, a piece without potentials. Needs a GPU;
//! skipped (with a note) when the planner cannot set up. The planner object is shared, since each
//! one owns an arena.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use crate::{GpuCountAndPlan, MemPlanner};
use zisk_common::{ChunkId, Plan};
use zisk_sm_mem_common::{MemAlignCheckPoint, MemModuleSegmentCheckPoint};

// The memory map of core/src/mem.rs, as count_and_plan.cuh carries it.
const INPUT_ADDR: u32 = 0x1000_0000;
const INPUT_SIZE: u32 = 1 << 31;
const ROM_ADDR: u32 = 0x9000_0000;
const ROM_SIZE: u32 = 1 << 27;
const RAM_ADDR: u32 = 0xA000_0000;
/// The largest gap two consecutive addresses of an immutable region may have (2^24 words), the
/// step of the BIOS anchor reads.
const ANCHOR_STEP: u32 = (1 << 24) * 8;

// Light records (mops_format.hpp): a header word per record, no values, no steps.
const TAG: u64 = 1 << 63;
const NO_VALUE: u64 = 1 << 58;
const NO_PAYLOAD: u64 = 1 << 62;
fn read8(addr: u32) -> u64 {
    TAG | (0x08 << 32) | addr as u64
}
fn write8(addr: u32) -> u64 {
    TAG | NO_VALUE | (0x18 << 32) | addr as u64
}
fn block_read(addr: u32, words: u64) -> u64 {
    TAG | NO_PAYLOAD | (0x0A << 32) | (words << 36) | addr as u64
}

fn planner() -> Option<&'static Mutex<GpuCountAndPlan>> {
    static PLANNER: OnceLock<Option<Mutex<GpuCountAndPlan>>> = OnceLock::new();
    PLANNER
        .get_or_init(|| {
            let gpu = GpuCountAndPlan::new();
            // SAFETY: a null buffer asks the planner to allocate its own arena.
            let ok = unsafe { gpu.setup(std::ptr::null_mut(), 0, 1, 0, -1, false) };
            if !ok {
                eprintln!("no usable GPU for the planner: the plan edge tests are skipped");
            }
            ok.then_some(Mutex::new(gpu))
        })
        .as_ref()
}

/// The CPU planner's plans for `chunks`, as the runner collects them.
fn cpu_plans(chunks: &[Vec<u64>]) -> Vec<Plan> {
    let p = MemPlanner::new();
    p.execute();
    for c in chunks {
        p.add_chunk(c.len() as u64, c.as_ptr() as *const c_void);
    }
    p.set_completed();
    let mut align = p.wait_mem_align_plans();
    p.wait();
    p.collect_plans(&mut align)
}

/// The GPU planner's plans for `chunks`, injected into a CPU planner as the runner does; `None`
/// when the device declined the block.
fn gpu_plans(gpu: &GpuCountAndPlan, chunks: &[Vec<u64>]) -> Option<Vec<Plan>> {
    gpu.reset();
    for c in chunks {
        assert!(gpu.add_chunk(c.len() as u64, c.as_ptr() as *const c_void), "the planner rejected a chunk");
    }
    let metas = gpu.run()?;
    // As the runner does on the device path: no CPU workers are started, the metas are injected.
    let p = MemPlanner::new();
    // SAFETY: `metas` is the planner's array, alive until the next `reset`, which comes after.
    let ok = unsafe { p.inject_gpu_metas_from_pointers(metas.as_ptr() as *const c_void, metas.len() as u32) };
    assert!(ok, "the metas were rejected");
    let mut align = gpu.build_align_plans();
    Some(p.collect_plans(&mut align))
}

/// What the two planners must agree on for one memory segment. The per-chunk checkpoints carry
/// conventions of their own (the device path writes the instance's bounds on every chunk and an
/// all-ones count where the CPU planner writes the chunk's own address and exact count, and it
/// counts the continuation access on an instance's first chunk; the collector reads both the
/// same), so the comparison is on the segment: the chunks it spans, its first and last address,
/// and the accesses it holds, with the continuation discounted.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Segment {
    air_id: usize,
    segment: usize,
    is_last: bool,
    first_chunk: Option<usize>,
    chunks: Vec<usize>,
    first_addr: u32,
    last_addr: u32,
    accesses: u64,
}

/// The plans as comparable segments, plus the MemAlign plans as text.
fn describe(plans: &[Plan], device: bool) -> (Vec<Segment>, Vec<String>) {
    let mut segments = Vec::new();
    let mut align = Vec::new();
    for plan in plans {
        let segment = plan.segment_id.map(usize::from).unwrap_or(0);
        let meta = plan.meta.as_ref().expect("a memory plan carries its checkpoints");
        if let Some(cp) = meta.downcast_ref::<MemModuleSegmentCheckPoint>() {
            let mut chunks: Vec<usize> = cp.chunks.keys().map(|c| c.0).collect();
            chunks.sort();
            let accesses: u64 = cp.chunks.values().map(|k| k.count as u64).sum();
            segments.push(Segment {
                air_id: plan.air_id,
                segment,
                is_last: cp.is_last_segment,
                first_chunk: cp.first_chunk_id.map(|c| c.0),
                chunks,
                first_addr: cp.chunks.values().map(|k| k.from_addr).min().unwrap_or(0),
                last_addr: cp.chunks.values().map(|k| k.to_addr).max().unwrap_or(0),
                accesses: accesses - u64::from(device && segment > 0),
            });
        } else if let Some(cps) = meta.downcast_ref::<std::collections::HashMap<ChunkId, MemAlignCheckPoint>>() {
            let chunks: BTreeMap<usize, String> = cps.iter().map(|(c, k)| (c.0, format!("{k:?}"))).collect();
            align.push(format!("align air {} seg {} {chunks:?}", plan.air_id, segment));
        } else {
            align.push(format!("air {} seg {} (opaque meta)", plan.air_id, segment));
        }
    }
    segments.sort();
    align.sort();
    (segments, align)
}

fn assert_same_plans(name: &str, chunks: &[Vec<u64>]) {
    let Some(gpu) = planner() else { return };
    let gpu = gpu.lock().unwrap_or_else(|e| e.into_inner());
    let device = gpu_plans(&gpu, chunks).unwrap_or_else(|| panic!("{name}: the device declined the block"));
    let host = cpu_plans(chunks);
    let (d, da) = describe(&device, true);
    let (h, ha) = describe(&host, false);
    assert_eq!(da, ha, "{name}: the MemAlign plans differ between the device and the host");
    assert_eq!(d, h, "{name}: the memory segments differ between the device and the host");
}

/// A few accesses to every region, the shape of a small block.
fn small_chunk(salt: u32) -> Vec<u64> {
    let mut c = Vec::new();
    for i in 0..64u32 {
        c.push(read8(ROM_ADDR + 8 * (i + salt)));
        c.push(read8(INPUT_ADDR + 8 * i));
        c.push(write8(RAM_ADDR + 8 * (i % 7)));
        c.push(read8(RAM_ADDR + 8 * (i % 7)));
    }
    c
}

/// The last word of the ROM and input regions is accessed, with enough input accesses for more
/// than one InputData instance, so the instance cuts of the input and RAM regions depend on a
/// prefix that the scan of the region before them must not have shifted. The anchor reads keep
/// every address gap within what an immutable region allows, as the BIOS does.
#[test]
fn the_last_word_of_a_region_leaves_the_next_region_prefix_intact() {
    let input_rows = (zisk_pil::InputDataTrace::<()>::NUM_ROWS * zisk_sm_mem_common::input_data_lanes_x_row()) as u32;
    let mut c0 = Vec::new();
    for i in 0..input_rows + input_rows / 4 {
        c0.push(read8(INPUT_ADDR + 8 * i));
    }
    for k in 1..INPUT_SIZE / ANCHOR_STEP {
        c0.push(read8(INPUT_ADDR + k * ANCHOR_STEP));
    }
    c0.push(read8(INPUT_ADDR + INPUT_SIZE - 8));
    c0.push(read8(ROM_ADDR));
    c0.push(read8(ROM_ADDR + ROM_SIZE - 8));
    for i in 0..1000u32 {
        c0.push(write8(RAM_ADDR + 8 * i));
        c0.push(read8(RAM_ADDR + 8 * i));
    }
    let chunks = vec![c0, small_chunk(1), small_chunk(2)];
    assert_same_plans("last words", &chunks);
}

/// A chunk without memory-ops words, after a block whose same chunk had accesses: the device's
/// count for that chunk must not be the previous block's.
#[test]
fn an_empty_chunk_after_a_block_that_filled_it() {
    let Some(gpu) = planner() else { return };
    {
        let gpu = gpu.lock().unwrap_or_else(|e| e.into_inner());
        let first = vec![small_chunk(1), small_chunk(2), small_chunk(3)];
        assert!(gpu_plans(&gpu, &first).is_some(), "the first block plans");
    }
    let second = vec![small_chunk(1), Vec::new(), small_chunk(3)];
    assert_same_plans("empty chunk", &second);
}

/// A chunk whose records carry no access: block reads of zero words. The piece has no potential
/// and must still go through the pipeline.
#[test]
fn a_piece_of_zero_count_block_records() {
    let empties: Vec<u64> = (0..16).map(|i| block_read(RAM_ADDR + 64 * i, 0)).collect();
    let chunks = vec![small_chunk(1), empties, small_chunk(2)];
    assert_same_plans("zero-count blocks", &chunks);
}
