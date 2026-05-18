//! [`SecondaryWitnessHandler`] — witness compute for non-ROM
//! `InstanceType::Instance` secondary state machines.

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};

use super::common::take_collectors_for_instance;
use crate::state::ExecutionState;
use crate::{ChunkDataCollector, WitnessGenerator};

/// Static-namespace handler for non-ROM secondary `Instance`s.
///
/// Runs per-chunk collection via the executor's `ChunkDataCollector`
/// when the collector slot is empty, then drains the recorded
/// collectors and forwards to the witness generator's
/// `compute_secn_witness`.
pub struct SecondaryWitnessHandler;

impl SecondaryWitnessHandler {
    /// Compute the witness for `global_id` (assumed to be a non-ROM
    /// `InstanceType::Instance` secondary).
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
    ) -> Result<()> {
        let secn_instances =
            state.instance_set.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
        let secn_instance = secn_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

        let needs_collection = !state
            .collector_store
            .inner
            .read()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .contains_key(&global_id);

        if needs_collection {
            collector
                .collect_single(pctx, state, global_id, secn_instance)
                .map_err(|e| anyhow::anyhow!("Collector error: {e}"))?;
        }

        let instance = &**secn_instance;
        let collectors = take_collectors_for_instance(state, global_id, instance.instance_type())?;
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
