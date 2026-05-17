//! [`RomAsmWitnessHandler`] — ROM witness compute on the **ASM** backend.
//!
//! Unlike the native variant, ASM doesn't run per-chunk collection
//! for ROM — the ASM ROM-histogram (RH) service supplies the data
//! out-of-band, so the collector slot is filled with `register_empty_collector`
//! before the witness generator runs.

use std::sync::Mutex;

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};

use crate::state::ExecutionState;
use crate::witness_handlers::common::{register_empty_collector, take_collectors_for_instance};
use crate::WitnessGenerator;

/// Static-namespace handler for the ASM-backend ROM witness path.
pub struct RomAsmWitnessHandler;

impl RomAsmWitnessHandler {
    /// Compute the witness for the ROM global id under the ASM
    /// backend: register an empty per-chunk collector slot, then call
    /// into the witness generator with the shared ROM trace buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
        trace_buffer_rom: &Mutex<Vec<F>>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
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
            // ASM ROM path: skip per-chunk collection — the RH service
            // supplies the data — but pin an empty slot so the rest of
            // the pipeline observes the "no-collection" state.
            register_empty_collector(state, global_id, airgroup_id, air_id)?;
        }

        let instance = &**secn_instance;
        let collectors = take_collectors_for_instance(state, global_id, instance.instance_type())?;
        let trace_buffer =
            std::mem::take(&mut *trace_buffer_rom.lock().map_err(|e| anyhow::anyhow!("{e}"))?);

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
