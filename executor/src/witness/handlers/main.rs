//! Handlers for computing witnesses for Main SM instances.

use fields::PrimeField64;
use proofman_common::BufferPool;

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};
use crate::{state::ExecutionState, WitnessGenerator};

/// Main SM handler witness computation.
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
    ) -> ExecutorResult<()> {
        // Clone the Arc under a short guard and take the trace buffer OUTSIDE
        // any lock: holding the read guard across the (long) witness
        // computation — or across a potentially blocking take_buffer — would
        // stall the incremental main advancement, which takes the write lock
        // from the emulator's reader thread while chunks are still streaming.
        let main_instance = {
            let main_instances =
                state.instance_set.main_instances.read_or_poison("main_instances")?;
            main_instances
                .get(&global_id)
                .cloned()
                .ok_or(ExecutorError::InstanceNotFound { global_id })?
        };

        generator.compute_main_witness(
            pctx,
            state,
            &main_instance,
            buffer_pool.take_buffer(),
            stats_scope_id,
        )
    }
}
