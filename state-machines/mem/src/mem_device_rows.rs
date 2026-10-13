//! Which memory airs' rows this block takes from the GPU planner, and the MemAlign instances'
//! device and slot paths (`ZISK_MEM_GPU_FILL`, see `mem_gpu_fill`).

use proofman_common::{
    AirInstance, FromTrace, GenericTrace, ProofCtx, ProofmanError, ProofmanResult,
};
use proofman_fields::PrimeField64;
use zisk_common::{gpu_fill_mode, GpuFillMode};
use zisk_pil::ZISK_AIRGROUP_ID;

fn family_bit(name: &str) -> u32 {
    match name {
        "ram" => zisk_common::MEM_ROWS_RAM,
        "rom" => zisk_common::MEM_ROWS_ROM,
        "input" => zisk_common::MEM_ROWS_INPUT,
        "align" => zisk_common::MEM_ROWS_ALIGN,
        _ => 0,
    }
}

/// Whether this block's rows of the memory air family `name` ("ram", "rom", "input", "align") come
/// from the GPU planner (on the device, or already in its host memory for the proofs); otherwise
/// the instances are collected and filled on the CPU.
pub(crate) fn rows_on_device(name: &str) -> bool {
    gpu_fill_mode().on_device() && zisk_common::mem_rows_served(family_bit(name))
}

/// `ZISK_MEM_GPU_FILL=slot` while the family's rows are still on the device: the instance is
/// committed by the prover's kernel. Once the arena went back, the rows come from the host copy.
pub(crate) fn slot_pending(name: &str) -> bool {
    gpu_fill_mode() == GpuFillMode::Slot
        && zisk_common::MEM_ROWS_ON_DEVICE.load(std::sync::atomic::Ordering::Acquire)
            & family_bit(name)
            != 0
}

/// The planner's family of the MemAlign airs in a slot op.
pub(crate) const SLOT_FAMILY_ALIGN: u32 = zisk_sm_mem_planner::MEM_FAMILY_ALIGN;

/// The memory airs the planner's kernel fills in slot mode: their instances stage one op and the
/// kernel copies the staged image into the commit slot; their proofs take the host copy.
pub fn mem_slot_witness_airs() -> Vec<proofman_common::GpuWitnessAir> {
    use proofman_common::{GpuWitnessAir, TraceLayout};
    use zisk_pil::*;
    let op_bytes = std::mem::size_of::<crate::mem_gpu_fill::MemSlotOp>() as u64;
    [
        MemTrace::<()>::AIR_ID,
        RomDataTrace::<()>::AIR_ID,
        InputDataTrace::<()>::AIR_ID,
        MemAlignTrace::<()>::AIR_ID,
        MemAlignLargeTrace::<()>::AIR_ID,
        MemAlignByteTrace::<()>::AIR_ID,
        MemAlignByteLargeTrace::<()>::AIR_ID,
        MemAlignReadByteTrace::<()>::AIR_ID,
        MemAlignReadByteLargeTrace::<()>::AIR_ID,
        MemAlignWriteByteTrace::<()>::AIR_ID,
    ]
    .into_iter()
    .map(|air_id| {
        GpuWitnessAir::new(
            ZISK_AIRGROUP_ID,
            air_id,
            op_bytes,
            op_bytes,
            TraceLayout::PackedCm1,
            zisk_sm_mem_planner::zisk_mem_witness_slot_kernel,
        )
        .with_host_trace()
        .with_planner_gpu()
    })
    .collect()
}

fn invalid(msg: String) -> ProofmanError {
    ProofmanError::InvalidParameters(msg)
}

/// A MemAlign instance in slot mode: the staged op the prover's kernel fills, with the air
/// values the plan determines (`air_values(padding)`). `plan_used`, the rows the plan counted, is
/// checked against what the device built so a divergence fails loudly before the commit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn align_slot_instance<F: PrimeField64>(
    pctx: &ProofCtx<F>,
    airgroup_id: usize,
    air_id: usize,
    segment: usize,
    row_size: usize,
    trace_buffer: Vec<F>,
    packed: bool,
    plan_used: Option<usize>,
    air_values: impl FnOnce(usize) -> Vec<F>,
) -> ProofmanResult<AirInstance<F>> {
    if !packed {
        return Err(invalid("ZISK_MEM_GPU_FILL=slot needs the packed MemAlign traces".into()));
    }
    let decl = pctx.gpu_witness_airs.get(airgroup_id, air_id).ok_or_else(|| {
        invalid(format!("ZISK_MEM_GPU_FILL=slot: air {air_id} has no GPU witness declaration"))
    })?;
    let n_rows = zisk_sm_mem_common::mem_align_air_rows(air_id)
        .ok_or_else(|| invalid(format!("air {air_id} is not a MemAlign air")))?;
    let mut padding = 0;
    if let Some(used) = plan_used {
        let device = zisk_sm_mem_planner::gpu_mem_witness_align_used(air_id, segment)
            .map_err(|e| invalid(format!("MemAlign air {air_id}[{segment}]: {e}")))?;
        if device as usize != used {
            return Err(invalid(format!(
                "MemAlign air {air_id}[{segment}]: the device built {device} rows, the plan counted {used}"
            )));
        }
        padding = n_rows - used;
    }
    crate::mem_gpu_fill::slot_instance(
        decl,
        SLOT_FAMILY_ALIGN,
        air_id,
        segment,
        n_rows,
        row_size,
        trace_buffer,
        air_values(padding),
    )
}

/// A MemAlign instance from the rows the GPU planner built: `make` turns the filled trace and the
/// rows its accesses use into the instance (with the air values the air has).
pub(crate) fn align_device_instance<
    F: PrimeField64,
    Row: proofman_common::trace::TraceRow,
    const N: usize,
    const A: usize,
>(
    air_id: usize,
    segment: usize,
    trace_buffer: Vec<F>,
    packed: bool,
    words_per_row: usize,
    make: impl FnOnce(&mut GenericTrace<Row, N, ZISK_AIRGROUP_ID, A>, usize) -> AirInstance<F>,
) -> ProofmanResult<AirInstance<F>> {
    if !packed {
        return Err(invalid("ZISK_MEM_GPU_FILL=arena needs the packed MemAlign traces".into()));
    }
    let mut trace = GenericTrace::<Row, N, ZISK_AIRGROUP_ID, A>::new_from_vec(trace_buffer)?;
    let used = zisk_sm_mem_planner::gpu_align_witness_fill(
        air_id,
        segment,
        crate::mem_trace_hash::rows_as_words_mut(&mut trace.buffer),
    )
    .map_err(|e| {
        invalid(format!(
            "MemAlign air {air_id}[{segment}] witness from the GPU planner failed: {e}"
        ))
    })?;
    align_dump(air_id, segment, crate::mem_trace_hash::rows_as_words(&trace.buffer), words_per_row);
    Ok(make(&mut trace, used))
}

/// The hash dump of a MemAlign instance's packed rows (`ZISK_MEM_TRACE_HASH_DIR`); the instance
/// has no air values.
pub(crate) fn align_dump(air_id: usize, segment: usize, rows: &[u64], words_per_row: usize) {
    let used_bits = crate::mem_gpu_fill::packed_used_bits(ZISK_AIRGROUP_ID, air_id)
        .unwrap_or(words_per_row * 64);
    crate::mem_trace_hash::dump_scalars(
        &format!("align{air_id}"),
        segment,
        rows,
        &[],
        words_per_row,
        used_bits,
    );
}

/// A MemAlign instance from a filled trace without air values.
pub(crate) fn plain_instance<
    F: PrimeField64,
    Row: proofman_common::trace::TraceRow,
    const N: usize,
    const A: usize,
>(
    trace: &mut GenericTrace<Row, N, ZISK_AIRGROUP_ID, A>,
) -> AirInstance<F> {
    AirInstance::new_from_trace(FromTrace::new(trace))
}
