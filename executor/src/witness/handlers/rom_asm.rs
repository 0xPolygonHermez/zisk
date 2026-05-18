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

use crate::ports::{Dctx, GlobalId};
use crate::state::ExecutionState;
use super::common::{register_empty_collector, take_collectors_for_instance};
use super::{RomWitnessHandler, SecnInstanceMap, SecnInstanceMapRef};
use crate::{ChunkDataCollector, WitnessGenerator};

/// Strategy implementor for the ASM-backend ROM witness path.
pub struct RomAsmWitnessHandler;

impl<F: PrimeField64> RomWitnessHandler<F> for RomAsmWitnessHandler {
    /// Compute the witness for the ROM global id under the ASM
    /// backend: register an empty per-chunk collector slot, then call
    /// into the witness generator with the shared ROM trace buffer.
    /// The `_collector` argument is unused — the ASM RH service supplies
    /// ROM histogram data out-of-band.
    fn dispatch(
        &self,
        generator: &WitnessGenerator,
        _collector: &ChunkDataCollector<F>,
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

    /// Pre-calculate hook for ASM ROM: unconditionally flag the gid not-ready.
    /// All other args are unused — the ASM RH service handles collection
    /// out-of-band, so there is nothing to enqueue or downcast here.
    fn pre_calculate<'a>(
        &self,
        registry: &dyn Dctx,
        _state: &ExecutionState<F>,
        _secn_instances: &'a SecnInstanceMap<F>,
        _instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
        _airgroup_id: usize,
        _air_id: usize,
    ) -> Result<()> {
        registry.set_witness_ready(GlobalId(global_id), false);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::fakes::FakeProofRegistry;
    use fields::Goldilocks;
    use std::collections::HashMap;

    type F = Goldilocks;

    #[test]
    fn pre_calculate_marks_gid_not_ready_unconditionally() {
        let registry = FakeProofRegistry::new();
        let state: ExecutionState<F> = ExecutionState::new();
        let secn_instances: SecnInstanceMap<F> = HashMap::new();
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        <RomAsmWitnessHandler as RomWitnessHandler<F>>::pre_calculate(
            &RomAsmWitnessHandler,
            &registry,
            &state,
            &secn_instances,
            &mut instances_to_collect,
            42,
            7,
            13,
        )
        .expect("pre_calculate must not error on empty inputs");

        // ASM path never enqueues; flips the gid not-ready.
        assert!(instances_to_collect.is_empty());
        assert_eq!(registry.witness_ready.borrow().get(&GlobalId(42)), Some(&false));
    }
}
