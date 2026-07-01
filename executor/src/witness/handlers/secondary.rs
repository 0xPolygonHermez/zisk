//! [`SecondaryWitnessHandler`] — witness compute for non-ROM `InstanceType::Instance`
//! secondary state machines.

use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};
use crate::state::ExecutionState;
use crate::{ChunkDataCollector, WitnessGenerator};

/// Secondary handler witness computation for non-ROM `InstanceType::Instance` secondaries.
pub struct SecondaryWitnessHandler;

impl SecondaryWitnessHandler {
    /// Compute the witness for `global_id` (assumed to be a non-ROM `InstanceType::Instance`).
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
        collector: &ChunkDataCollector<F>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        buffer_pool: &dyn BufferPool<F>,
        stats_scope_id: u64,
    ) -> ExecutorResult<()> {
        let secn_instances = state.instance_set.secn_instances.read_or_poison("secn_instances")?;
        let secn_instance =
            secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;

        let needs_collection = !state
            .collector_store
            .inner
            .read_or_poison("collector_store")?
            .contains_key(&global_id);

        let instance = &**secn_instance;
        if needs_collection {
            collector.collect_single(pctx, state, global_id, instance)?;
        }

        let collectors = state.take_collectors_for_instance(global_id, instance.instance_type())?;
        let trace_buffer = buffer_pool.take_buffer();

        generator.compute_secn_witness(
            pctx,
            sctx,
            state,
            global_id,
            instance,
            collectors,
            trace_buffer,
            stats_scope_id,
        )
    }
}
