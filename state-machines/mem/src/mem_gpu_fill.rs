//! The `Mem` witness from the accesses the GPU planner retained (`ZISK_MEM_GPU_FILL`), and the
//! op the prover's kernel takes in slot mode.

use zisk_common::SegmentId;
use zisk_pil::PACKED_INFO;
use zisk_sm_mem_common::{RAM_W_ADDR_END, RAM_W_ADDR_INIT};

use crate::mem_sm::{split_last_step, split_padding_size, MemFillOutput, MemPreviousSegment};

/// A fill's observer: the filled rows as words and the fill's scalars, before the air instance is
/// built. The arena check compares them with the device rows.
pub(crate) struct ArenaFillReport {
    pub res: zisk_sm_mem_planner::RamFillResult,
    pub previous_segment: MemPreviousSegment,
    pub out: MemFillOutput,
}

/// Fills `rows` (`n_rows` packed rows as words) of RAM instance `segment_id` from the accesses the
/// GPU planner retained for this block.
pub(crate) fn arena_fill_packed_rows(
    rows: &mut [u64],
    n_rows: usize,
    segment_id: SegmentId,
) -> Result<ArenaFillReport, String> {
    let res = zisk_sm_mem_planner::gpu_ram_witness_fill(
        usize::from(segment_id) as u32,
        rows,
        n_rows as u32,
    )?;
    let (previous_segment, out) = mem_report(&res, n_rows);
    Ok(ArenaFillReport { res, previous_segment, out })
}

/// The scalars a filled Mem instance's air values are built from, out of the planner's result.
pub(crate) fn mem_report(
    res: &zisk_sm_mem_planner::RamFillResult,
    n_rows: usize,
) -> (MemPreviousSegment, MemFillOutput) {
    let previous_segment =
        MemPreviousSegment { addr: res.prev_addr_w, step: res.prev_step, value: res.prev_value };
    // The padding (`MemPadding` in mem_sm.rs) reads the last word of the region: when there is
    // any, that word is the LAST lane the air values describe, holding the last access's value
    // only if that access already sat there.
    let num_slots = n_rows * zisk_sm_mem_common::mem_lanes_x_row();
    let padding_size = (num_slots - res.n_lanes as usize) as u32;
    let (last_addr, last_value) = if padding_size > 0 {
        let value = if res.last_addr_w == RAM_W_ADDR_END { res.last_value } else { 0 };
        (RAM_W_ADDR_END, value)
    } else {
        (res.last_addr_w, res.last_value)
    };
    let distance_base = res.prev_addr_w - RAM_W_ADDR_INIT;
    let distance_end = RAM_W_ADDR_END - last_addr;
    let (padding_size_chunks, padding_size_to_max_chunks) =
        split_padding_size(padding_size, (num_slots - 1) as u32);
    let out = MemFillOutput {
        last_addr,
        last_step: res.last_step,
        last_value: [last_value as u32, (last_value >> 32) as u32],
        distance_base: [distance_base as u16, (distance_base >> 16) as u16],
        distance_end: [distance_end as u16, (distance_end >> 16) as u16],
        last_step_chunks: split_last_step(res.last_step),
        padding_size,
        padding_size_chunks,
        padding_size_to_max_chunks,
    };
    (previous_segment, out)
}

/// The bits a packed row of the air uses, counted from its first word; `None` when not packed.
pub(crate) fn packed_used_bits(airgroup_id: usize, air_id: usize) -> Option<usize> {
    PACKED_INFO
        .iter()
        .find(|(ag, a, _)| *ag == airgroup_id && *a == air_id)
        .filter(|(_, _, c)| c.is_packed)
        .map(|(_, _, c)| c.unpack_info.iter().map(|&w| w as usize).sum())
}

/// The kernel input of one memory instance (`MemSlotOp` of the planner, byte for byte).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemSlotOp {
    pub family: u32,
    pub air_id: u32,
    pub segment: u32,
    pub n_rows: u32,
}
// SAFETY: `#[repr(C)]`, four `u32` fields, no padding, no pointers: the planner's `MemSlotOp`.
unsafe impl proofman_common::GpuWitnessOp for MemSlotOp {}
impl From<&MemSlotOp> for MemSlotOp {
    fn from(op: &MemSlotOp) -> Self {
        *op
    }
}

/// The families the kernel input names.
pub(crate) const SLOT_FAMILY_RAM: u32 = 0;
pub(crate) const SLOT_FAMILY_ROM: u32 = 1;
pub(crate) const SLOT_FAMILY_INPUT: u32 = 2;

/// An instance the prover's kernel fills into its slot: the staged op in the trace buffer and the
/// air values, which the device resolved ahead. `decl` is the prover's declaration of the air.
#[allow(clippy::too_many_arguments)]
pub(crate) fn slot_instance<F: proofman_fields::PrimeField64>(
    decl: &proofman_common::GpuWitnessAir,
    family: u32,
    air_id: usize,
    segment: usize,
    n_rows: usize,
    n_cols: usize,
    trace_buffer: Vec<F>,
    air_values: Vec<F>,
) -> proofman_common::ProofmanResult<proofman_common::AirInstance<F>> {
    let op =
        MemSlotOp { family, air_id: air_id as u32, segment: segment as u32, n_rows: n_rows as u32 };
    let (mut instance, _) = proofman_common::stage_gpu_witness::<F, MemSlotOp, MemSlotOp>(
        decl,
        n_rows,
        n_cols,
        trace_buffer,
        &[vec![op]],
    )?;
    instance.airvalues = air_values;
    Ok(instance)
}
