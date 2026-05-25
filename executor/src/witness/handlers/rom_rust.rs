//! ROM pre-calculate hook on the **Rust** backend.

use fields::PrimeField64;
use sm_rom::RomInstance;

use super::{SecnInstanceMap, SecnInstanceMapRef};
use crate::error::{ExecutorError, ExecutorResult};
use crate::ports::{Dctx, GlobalId};
use crate::state::ExecutionState;

/// Pre-calculate hook for Rust ROM.
pub(crate) fn pre_calculate<'a, F: PrimeField64>(
    registry: &dyn Dctx,
    state: &ExecutionState<F>,
    secn_instances: &'a SecnInstanceMap<F>,
    instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
    global_id: usize,
    airgroup_id: usize,
    air_id: usize,
) -> ExecutorResult<()> {
    let gid = GlobalId(global_id);
    let secn_instance =
        secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;
    let rom_instance = secn_instance.as_any().downcast_ref::<RomInstance>().ok_or(
        ExecutorError::InstanceTypeMismatch { global_id, air_id, expected: "RomInstance" },
    )?;

    if rom_instance.skip_collector() {
        state.register_empty_collector(global_id, airgroup_id, air_id)?;
        registry.set_witness_ready(gid, true);
    } else {
        instances_to_collect.insert(global_id, &**secn_instance);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::fakes::FakeProofRegistry;
    use asm_runner::{AsmRHData, AsmRunnerRH};
    use fields::Goldilocks;
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
        if let Some(rh_data) = rh_data {
            Box::new(RomInstance::new_asm(Arc::new(ZiskRom::default()), ictx, rh_data))
        } else {
            Box::new(RomInstance::new_rust(
                Arc::new(ZiskRom::default()),
                ictx,
                Arc::new(Vec::<AtomicU64>::new()),
            ))
        }
    }

    fn run_pre_calculate<'a>(
        registry: &FakeProofRegistry,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
    ) -> ExecutorResult<()> {
        pre_calculate(
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
        let rh_data = AsmRunnerRH::new(AsmRHData::new(0, Vec::new()));
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
        assert!(err.to_string().contains(&format!("instance not found for global_id={GID}")));
    }
}
