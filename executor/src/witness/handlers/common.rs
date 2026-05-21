//! Helpers shared across the per-category witness handlers.
//!
//! Pulled out of [`crate::WitnessRouter`] in step 4.3 so each handler
//! module stays focused on its own air-id category.

use fields::PrimeField64;
use zisk_common::{BusDevice, InstanceType, Stats};

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};
use crate::state::ExecutionState;

/// Drains the per-chunk collectors recorded for `global_id` from
/// `state.collector_store`. Returns an empty list when the instance
/// is a `Table` (tables don't have per-chunk collectors).
///
/// # Errors
/// * `Instance`: errors if the global_id has no recorded entry, or if
///   any chunk slot is `None`.
#[allow(clippy::type_complexity)]
pub(super) fn take_collectors_for_instance<F: PrimeField64>(
    state: &ExecutionState<F>,
    global_id: usize,
    instance_type: InstanceType,
) -> ExecutorResult<Vec<(usize, Box<dyn BusDevice<u64>>)>> {
    match instance_type {
        InstanceType::Instance => {
            let mut guard = state.collector_store.inner.write_or_poison("collector_store")?;

            let collectors = guard
                .remove(&global_id)
                .ok_or(ExecutorError::MissingIndexEntry { global_id, index: "collector_store" })?;

            collectors
                .into_iter()
                .enumerate()
                .map(|(idx, opt)| {
                    opt.ok_or_else(|| {
                        ExecutorError::Internal(format!(
                            "collector at index {idx} for global_id {global_id} is None"
                        ))
                    })
                })
                .collect::<ExecutorResult<Vec<_>>>()
        }
        InstanceType::Table => Ok(vec![]),
    }
}

/// Records an empty per-chunk collector slot for an instance that
/// skips per-chunk collection (today: the ASM ROM path). Also pins a
/// `Stats::new_no_collection` entry so observability reflects the
/// "skipped collection" state.
pub(crate) fn register_empty_collector<F: PrimeField64>(
    state: &ExecutionState<F>,
    global_id: usize,
    airgroup_id: usize,
    air_id: usize,
) -> ExecutorResult<()> {
    let stats = Stats::new_no_collection(airgroup_id, air_id);

    state.collector_store.inner.write_or_poison("collector_store")?.insert(global_id, Vec::new());
    state.stats.insert_witness_stats(global_id, stats);

    Ok(())
}
