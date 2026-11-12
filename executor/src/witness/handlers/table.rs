//! [`TableWitnessHandler`] — witness compute for `InstanceType::Table` secondary state machines.

use proofman_common::{ProofCtx, SetupCtx};
use proofman_fields::PrimeField64;

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};
use crate::state::ExecutionState;
use crate::WitnessGenerator;

/// Secondary handler witness computation for `InstanceType::Table` secondaries.
pub struct TableWitnessHandler;

impl TableWitnessHandler {
    /// Compute the witness for a `InstanceType::Table` secondary.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        stats_scope_id: u64,
    ) -> ExecutorResult<()> {
        let secn_instances = state.instance_set.secn_instances.read_or_poison("secn_instances")?;
        let secn_instance =
            secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;

        let instance = &**secn_instance;
        // Tables have no per-chunk collectors and build their own trace: a pooled buffer would
        // only be dropped.
        generator
            .compute_secn_witness(
                pctx,
                sctx,
                state,
                global_id,
                instance,
                Vec::new(),
                Vec::new(),
                stats_scope_id,
            )
            .map_err(|(e, _)| e)
    }
}
