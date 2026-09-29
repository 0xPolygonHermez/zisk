//! ROM pre-calculate hook on the **Rust** backend.

use proofman_fields::PrimeField64;
use zisk_sm_rom::RomInstance;

use super::{SecnInstanceMap, SecnInstanceMapRef};
use crate::error::{ExecutorError, ExecutorResult};

/// Pre-calculate hook for Rust ROM: queues the instance for collection.
///
/// Rejects an ASM-backend instance rather than serving it. The Rust emulator parks
/// no ROM histogram, so one reaching this path was built from a histogram a previous
/// job left armed — see [`ExecutorError::RomBackendStale`].
pub(crate) fn pre_calculate<'a, F: PrimeField64>(
    secn_instances: &'a SecnInstanceMap<F>,
    instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
    global_id: usize,
    air_id: usize,
) -> ExecutorResult<()> {
    let secn_instance =
        secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;
    let rom_instance =
        crate::sm::downcast::<F, RomInstance>(&**secn_instance, air_id, global_id, "RomInstance")?;

    if rom_instance.skip_collector() {
        return Err(ExecutorError::RomBackendStale { global_id });
    }
    instances_to_collect.insert(global_id, &**secn_instance);
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use proofman_fields::Goldilocks;
    use std::collections::HashMap;
    use std::sync::{atomic::AtomicU64, Arc};
    use zisk_asm_runner::{AsmRHData, AsmRunnerRH};
    use zisk_common::LateValue;
    use zisk_common::{CheckPoint, Instance, InstanceCtx, InstanceType, Plan};
    use zisk_core::ZiskRom;

    type F = Goldilocks;

    pub(crate) const GID: usize = 42;
    const AIRGROUP_ID: usize = 7;
    pub(crate) const AIR_ID: usize = 13;

    /// A ROM instance in either backend, built the way `RomSM::build_instance` builds
    /// it. Shared with the parent module's tests — the only other place that needs one.
    pub(crate) fn make_rom_instance(rh_data: Option<AsmRunnerRH>) -> Box<dyn Instance<F>> {
        let plan =
            Plan::new(AIRGROUP_ID, AIR_ID, None, InstanceType::Instance, CheckPoint::None, None);
        let ictx = InstanceCtx::new(GID, plan);
        if let Some(rh_data) = rh_data {
            let cell = Arc::new(LateValue::ready(zisk_sm_rom::RH_LABEL, rh_data));
            Box::new(RomInstance::new_asm(Arc::new(ZiskRom::default()), ictx, cell))
        } else {
            Box::new(RomInstance::new_rust(
                Arc::new(ZiskRom::default()),
                ictx,
                Arc::new(Vec::<AtomicU64>::new()),
            ))
        }
    }

    fn run_pre_calculate<'a>(
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
    ) -> ExecutorResult<()> {
        pre_calculate(secn_instances, instances_to_collect, GID, AIR_ID)
    }

    #[test]
    fn pre_calculate_enqueues_a_rust_backend_instance() {
        let mut secn_instances: SecnInstanceMap<F> = HashMap::new();
        secn_instances.insert(GID, make_rom_instance(None));
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        run_pre_calculate(&secn_instances, &mut instances_to_collect)
            .expect("pre_calculate must succeed on a Rust-backend RomInstance");

        assert!(instances_to_collect.contains_key(&GID));
    }

    #[test]
    fn pre_calculate_rejects_an_asm_backend_instance() {
        // What a previous job's histogram, never drained, makes `RomSM` build.
        let rh_data = AsmRunnerRH::new(AsmRHData::new(0, Vec::new(), Vec::new()));
        let mut secn_instances: SecnInstanceMap<F> = HashMap::new();
        secn_instances.insert(GID, make_rom_instance(Some(rh_data)));
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        let err = run_pre_calculate(&secn_instances, &mut instances_to_collect)
            .expect_err("a stale histogram must not become this execution's ROM witness");

        assert!(matches!(err, ExecutorError::RomBackendStale { global_id: GID }), "got {err:?}");
        assert!(instances_to_collect.is_empty());
    }

    #[test]
    fn pre_calculate_errors_when_instance_missing() {
        let secn_instances: SecnInstanceMap<F> = HashMap::new(); // empty
        let mut instances_to_collect: SecnInstanceMapRef<'_, F> = HashMap::new();

        let err = run_pre_calculate(&secn_instances, &mut instances_to_collect)
            .expect_err("must err when the gid isn't present in the map");
        assert!(err.to_string().contains(&format!("instance not found for global_id={GID}")));
    }
}
