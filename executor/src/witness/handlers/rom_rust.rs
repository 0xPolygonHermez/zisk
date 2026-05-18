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

use super::common::{register_empty_collector, take_collectors_for_instance};
use super::{RomWitnessHandler, SecnInstanceMap, SecnInstanceMapRef};
use crate::ports::{Dctx, GlobalId};
use crate::state::ExecutionState;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::fakes::FakeProofRegistry;
    use asm_runner::{AsmRHData, AsmRunnerRH};
    use fields::Goldilocks;
    use sm_rom::RomInstance;
    use std::collections::HashMap;
    use std::sync::{atomic::AtomicU64, Arc};
    use zisk_common::{CheckPoint, Instance, InstanceCtx, InstanceType, Plan};
    use zisk_core::ZiskRom;

    type F = Goldilocks;

    const GID: usize = 42;
    const AIRGROUP_ID: usize = 7;
    const AIR_ID: usize = 13;

    fn make_rom_instance(rh_data: Option<AsmRunnerRH>) -> Box<dyn Instance<F>> {
        let plan =
            Plan::new(AIRGROUP_ID, AIR_ID, None, InstanceType::Instance, CheckPoint::None, None);
        let ictx = InstanceCtx::new(GID, plan);
        Box::new(RomInstance::new(
            Arc::new(ZiskRom::default()),
            ictx,
            Arc::new(Vec::<AtomicU64>::new()),
            Arc::new(Vec::<AtomicU64>::new()),
            rh_data,
        ))
    }

    fn run_pre_calculate<'a>(
        registry: &FakeProofRegistry,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
    ) -> Result<()> {
        <RomNativeWitnessHandler as RomWitnessHandler<F>>::pre_calculate(
            &RomNativeWitnessHandler,
            registry,
            state,
            secn_instances,
            instances_to_collect,
            GID,
            AIRGROUP_ID,
            AIR_ID,
        )
    }

    #[test]
    fn pre_calculate_enqueues_when_skip_collector_false() {
        // Non-ASM RomInstance (no rh_data, no counter_stats) → skip_collector() == false.
        let mut secn_instances: SecnInstanceMap<F> = HashMap::new();
        secn_instances.insert(GID, make_rom_instance(None));

        let registry = FakeProofRegistry::new();
        let state: ExecutionState<F> = ExecutionState::new();
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        run_pre_calculate(&registry, &state, &secn_instances, &mut instances_to_collect)
            .expect("pre_calculate must succeed on a non-ASM RomInstance");

        // The instance is queued for collection; collector store is not touched;
        // the registry's witness-ready map is untouched.
        assert!(instances_to_collect.contains_key(&GID));
        assert!(registry.witness_ready.borrow().get(&GlobalId(GID)).is_none());
        assert!(state.collector_store.inner.read().unwrap().get(&GID).is_none());
    }

    #[test]
    fn pre_calculate_skips_and_marks_ready_when_skip_collector_true() {
        // ASM-execution RomInstance (rh_data is Some) → skip_collector() == true.
        let rh_data = AsmRunnerRH::new(AsmRHData::new(0, Vec::new(), Vec::new()));
        let mut secn_instances: SecnInstanceMap<F> = HashMap::new();
        secn_instances.insert(GID, make_rom_instance(Some(rh_data)));

        let registry = FakeProofRegistry::new();
        let state: ExecutionState<F> = ExecutionState::new();
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        run_pre_calculate(&registry, &state, &secn_instances, &mut instances_to_collect)
            .expect("pre_calculate must succeed when skip_collector returns true");

        // Nothing is queued for collection; the collector slot is filled (empty Vec)
        // and the gid is flipped ready on the registry.
        assert!(instances_to_collect.is_empty());
        assert_eq!(registry.witness_ready.borrow().get(&GlobalId(GID)), Some(&true));
        let store = state.collector_store.inner.read().unwrap();
        let slot = store.get(&GID).expect("empty collector slot must be registered");
        assert!(slot.is_empty());
    }

    #[test]
    fn pre_calculate_errors_when_instance_missing() {
        let secn_instances: SecnInstanceMap<F> = HashMap::new(); // empty
        let registry = FakeProofRegistry::new();
        let state: ExecutionState<F> = ExecutionState::new();
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        let err = run_pre_calculate(&registry, &state, &secn_instances, &mut instances_to_collect)
            .expect_err("must err when the gid isn't present in the map");
        assert!(err.to_string().contains(&format!("Instance not found: global_id={GID}")));
    }
}
