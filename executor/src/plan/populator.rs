//! Build + store instances into [`crate::ExecutionState`] and configure
//! them on the proof context.
//!
//! Pair to [`super::InstanceAssigner`]: that one stamps plans with
//! global ids on the proof context; this one materialises them into
//! state and wires their checkpoints / SM configuration.

use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_main::MainInstance;
use std::collections::hash_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use zisk_common::{CheckPoint, Instance, InstanceCtx, InstanceType, Plan};

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};
use crate::ports::{GlobalId, ProofRegistry};
use crate::AirClassifier;
use crate::{state::ExecutionState, StaticSMBundle};

/// Instance populator for the ZiskExecutor. Responsible for materialising planned instances
/// into the execution state and configuring them on the proof context.
pub struct InstancePopulator<F: PrimeField64> {
    sm_bundle: Arc<StaticSMBundle<F>>,
}

impl<F: PrimeField64> InstancePopulator<F> {
    /// Creates a new `InstancePopulator` backed by `sm_bundle`.
    pub fn new(sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self { sm_bundle }
    }

    /// Build a main-SM instance for the given `(plan, global_id)`.
    ///
    /// # Arguments
    /// * `plan` - Plan for the instance to build.
    /// * `global_id` - Global ID to stamp the instance with.
    ///
    /// # Returns
    /// The built `MainInstance<F>`.
    fn new_main(&self, plan: Plan, global_id: usize) -> MainInstance<F> {
        MainInstance::new(InstanceCtx::new(global_id, plan), self.sm_bundle.get_std())
    }

    /// Build a secondary-SM instance for the given `(plan, global_id)`.
    ///
    /// # Arguments
    /// * `plan` - Plan for the instance to build.
    /// * `global_id` - Global ID to stamp the instance with.
    ///
    /// # Returns
    /// The built secondary instance.
    ///
    /// # Errors
    /// Returns an error if the instance cannot be built from the plan.
    fn new_secn(&self, plan: Plan, global_id: usize) -> ExecutorResult<Box<dyn Instance<F>>> {
        self.sm_bundle.build_instance(InstanceCtx::new(global_id, plan))
    }

    /// Populates main instances in the execution state.
    ///
    /// # Arguments
    /// * `registry` - Proof-context surface (used to gate
    /// * `state` - Execution state to populate.
    /// * `assignments` - Vector of (global_id, plan) pairs.
    ///
    /// # Errors
    /// Returns an error if any instance creation fails or if any plan is missing a `global_id`.
    pub fn populate_main_instances(
        &self,
        registry: &dyn ProofRegistry,
        state: &ExecutionState<F>,
        assignments: Vec<(usize, Plan)>,
    ) -> ExecutorResult<()> {
        let mut main_instances =
            state.instance_set.main_instances.write_or_poison("main_instances")?;
        for (global_id, plan) in assignments {
            main_instances.entry(global_id).or_insert_with(|| self.new_main(plan, global_id));

            let gid = GlobalId(global_id);
            if registry.is_my_process_instance(gid)? {
                registry.set_witness_ready(gid, false);
            }
        }

        Ok(())
    }

    /// Populates secondary instances in the execution state.
    ///
    /// # Arguments
    /// * `state` - Execution state to populate.
    /// * `plans` - Plans to populate (must have `global_id` stamped).
    ///
    /// # Errors
    /// Returns an error if any instance creation fails or if any plan is missing a `global_id`.
    pub fn populate_secn_instances(
        &self,
        state: &ExecutionState<F>,
        plans: Vec<Plan>,
    ) -> ExecutorResult<()> {
        let mut secn_instances =
            state.instance_set.secn_instances.write_or_poison("secn_instances")?;
        for plan in plans {
            let global_id =
                plan.global_id.ok_or(ExecutorError::SecnPlanMissing { phase: "populate" })?;
            if let Entry::Vacant(e) = secn_instances.entry(global_id) {
                let instance = self.new_secn(plan, global_id)?;
                e.insert(instance);
            }
        }

        Ok(())
    }

    /// Configures secondary state machine instances based on planning.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `plannings` - Map of SM ID to plans.
    pub fn configure_sm_instances(
        &self,
        pctx: &ProofCtx<F>,
        plannings: &BTreeMap<usize, Vec<Plan>>,
    ) {
        self.sm_bundle.configure_instances(pctx, plannings);
    }

    /// Configures checkpoints for secondary instances.
    ///
    /// # Arguments
    /// * `registry` - Proof-context surface for instance-info lookup and chunk assignment.
    /// * `state` - Execution state containing the instances.
    /// * `global_ids` - Global IDs of secondary instances to configure.
    /// 
    /// # Errors
    /// Returns an error if any instance is missing from state or if any registry lookup fails.
    pub fn configure_checkpoints(
        &self,
        registry: &dyn ProofRegistry,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) -> ExecutorResult<()> {
        let secn_instances = state.instance_set.secn_instances.read_or_poison("secn_instances")?;

        for &global_id in global_ids {
            let instance = secn_instances
                .get(&global_id)
                .ok_or(ExecutorError::InstanceNotFound { global_id })?;

            instance.reset();

            if instance.instance_type() == InstanceType::Instance {
                let checkpoint = instance.check_point();
                let chunks: Vec<usize> = match checkpoint {
                    CheckPoint::None => vec![],
                    CheckPoint::Single(chunk_id) => vec![chunk_id.as_usize()],
                    CheckPoint::Multiple(chunk_ids) => {
                        chunk_ids.iter().map(|id| id.as_usize()).collect()
                    }
                };

                let gid = GlobalId(global_id);
                let info = registry.instance_info(gid)?;
                let is_memory_related = AirClassifier::is_memory_related(info.air_id);
                registry.set_chunks(gid, &chunks, is_memory_related);
            }
        }

        Ok(())
    }
}
