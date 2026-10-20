//! Whether this block's memory rows come from the GPU planner, and the MemAlign instances' device
//! path (`ZISK_MEM_GPU_FILL`, see `mem_gpu_fill`).

use proofman_common::{AirInstance, FromTrace, GenericTrace, ProofmanError};
use proofman_fields::PrimeField64;
use zisk_common::WitnessError;
use zisk_pil::ZISK_AIRGROUP_ID;

/// Whether this block's rows of the memory airs come from the GPU planner; otherwise the
/// instances are collected and filled on the CPU.
pub(crate) fn rows_on_device() -> bool {
    zisk_common::device_mem_witness_active() && zisk_common::mem_rows_on_device()
}

fn invalid(msg: String) -> ProofmanError {
    ProofmanError::InvalidParameters(msg)
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
    make: impl FnOnce(&mut GenericTrace<Row, N, ZISK_AIRGROUP_ID, A>, usize) -> AirInstance<F>,
) -> Result<AirInstance<F>, WitnessError<F>> {
    if !packed {
        return Err(WitnessError::with_buffer(
            invalid("the device memory witness needs the packed MemAlign traces".into()),
            trace_buffer,
        ));
    }
    let mut trace = GenericTrace::<Row, N, ZISK_AIRGROUP_ID, A>::new_from_vec(trace_buffer)?;
    let used = zisk_sm_mem_planner::gpu_align_witness_fill(
        air_id,
        segment,
        crate::mem_gpu_fill::rows_as_words_mut(&mut trace.buffer),
    )
    .map_err(|e| {
        let error = invalid(format!(
            "MemAlign air {air_id}[{segment}] witness from the GPU planner failed: {e}"
        ));
        WitnessError::with_buffer(error, trace.get_buffer())
    })?;
    #[cfg(feature = "mem-witness-test")]
    align_dump(
        air_id,
        segment,
        crate::mem_gpu_fill::rows_as_words(&trace.buffer),
        std::mem::size_of::<Row>() / 8,
    );
    Ok(make(&mut trace, used))
}

/// The hash dump of a MemAlign instance's packed rows (`ZISK_MEM_TRACE_HASH_DIR`); the instance
/// has no air values.
#[cfg(feature = "mem-witness-test")]
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
