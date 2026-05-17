//! [`TableWitnessHandler`] — witness compute for
//! `InstanceType::Table` secondary state machines.
//!
//! Tables have no per-chunk collection; the handler skips straight
//! to the witness generator with an empty collector list and a
//! freshly-allocated trace buffer.

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};

use crate::state::ExecutionState;
use crate::WitnessGenerator;

/// Static-namespace handler for `InstanceType::Table` secondary
/// instances.
pub struct TableWitnessHandler;

impl TableWitnessHandler {
    /// Compute the witness for a table instance.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
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

        let instance = &**secn_instance;
        let collectors = Vec::new(); // Tables have no per-chunk collectors.
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
