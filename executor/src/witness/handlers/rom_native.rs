//! [`RomNativeWitnessHandler`] — ROM witness compute on the **native**
//! (Rust-emulator) backend.
//!
//! Mirrors [`super::secondary::SecondaryWitnessHandler`]
//! but uses the router's shared ROM trace buffer (single allocation
//! reused across runs) instead of pulling a fresh buffer from the
//! per-call pool.

use std::sync::Mutex;

use anyhow::Result;
use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};
use sm_rom::RomInstance;

use crate::ports::{Dctx, GlobalId};
use crate::state::ExecutionState;
use super::common::{register_empty_collector, take_collectors_for_instance};
use super::{RomWitnessHandler, SecnInstanceMap, SecnInstanceMapRef};
use crate::{ChunkDataCollector, WitnessGenerator};

/// Strategy implementor for the native-backend ROM witness path.
pub struct RomNativeWitnessHandler;

impl<F: PrimeField64> RomWitnessHandler<F> for RomNativeWitnessHandler {
    /// Compute the witness for the ROM global id under the native
    /// backend: run per-chunk collection if it hasn't already happened
    /// (the pre-calculate path may have done it), then drain the
    /// collectors and call into the witness generator with the shared
    /// ROM trace buffer. `airgroup_id`/`air_id` are unused on this path
    /// — the secondary instance carries them implicitly.
    fn dispatch(
        &self,
        generator: &WitnessGenerator,
        collector: &ChunkDataCollector<F>,
        trace_buffer_rom: &Mutex<Vec<F>>,
        state: &ExecutionState<F>,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        global_id: usize,
        _airgroup_id: usize,
        _air_id: usize,
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

    /// Pre-calculate hook for native ROM: downcasts the secondary
    /// instance to `RomInstance`. If `skip_collector()`, registers an
    /// empty collector and flips the gid to ready; otherwise enqueues
    /// the instance for per-chunk collection.
    fn pre_calculate<'a>(
        &self,
        registry: &dyn Dctx,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) -> Result<()> {
        let gid = GlobalId(global_id);
        let secn_instance = secn_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;
        let rom_instance =
            secn_instance.as_any().downcast_ref::<RomInstance>().ok_or_else(|| {
                anyhow::anyhow!("Downcast failed: instance {global_id} to RomInstance")
            })?;

        if rom_instance.skip_collector() {
            register_empty_collector(state, global_id, airgroup_id, air_id)?;
            registry.set_witness_ready(gid, true);
        } else {
            instances_to_collect.insert(global_id, secn_instance);
        }
        Ok(())
    }
}
