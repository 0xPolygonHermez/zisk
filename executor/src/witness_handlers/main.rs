//! [`MainWitnessHandler`] — witness compute for main-SM instances.

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::BufferPool;

use crate::{state::ExecutionState, WitnessGenerator};

/// Static-namespace handler for main-SM witness computation.
///
/// Looks up the main instance from the executor's
/// [`crate::InstanceSet`] and forwards to
/// [`WitnessGenerator::compute_main_witness`].
pub struct MainWitnessHandler;

impl MainWitnessHandler {
    /// Compute the witness for `global_id`.
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
        state: &ExecutionState<F>,
        pctx: &proofman_common::ProofCtx<F>,
        global_id: usize,
        buffer_pool: &dyn BufferPool<F>,
        stats_scope_id: u64,
    ) -> Result<()> {
        let main_instances =
            state.instance_set.main_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
        let main_instance = main_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

        generator.compute_main_witness(
            pctx,
            state,
            main_instance,
            buffer_pool.take_buffer(),
            stats_scope_id,
        )
    }
}
