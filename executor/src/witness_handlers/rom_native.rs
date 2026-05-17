//! [`RomNativeWitnessHandler`] — ROM witness compute on the **native**
//! (Rust-emulator) backend.
//!
//! Mirrors [`crate::witness_handlers::secondary::SecondaryWitnessHandler`]
//! but uses the router's shared ROM trace buffer (single allocation
//! reused across runs) instead of pulling a fresh buffer from the
//! per-call pool.

use std::sync::Mutex;

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};

use crate::state::ExecutionState;
use crate::witness_handlers::common::take_collectors_for_instance;
use crate::{ChunkDataCollector, WitnessGenerator};

/// Static-namespace handler for the native-backend ROM witness path.
pub struct RomNativeWitnessHandler;

impl RomNativeWitnessHandler {
    /// Compute the witness for the ROM global id under the native
    /// backend: run per-chunk collection if it hasn't already happened
    /// (the pre-calculate path may have done it), then drain the
    /// collectors and call into the witness generator with the shared
    /// ROM trace buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch<F: PrimeField64>(
        generator: &WitnessGenerator,
        collector: &ChunkDataCollector<F>,
        trace_buffer_rom: &Mutex<Vec<F>>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
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
