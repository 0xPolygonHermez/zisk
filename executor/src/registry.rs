//! Instance registry component.
//!
//! This module handles the creation and lifecycle management of main and secondary
//! state machine instances.

use fields::PrimeField64;
use proofman_common::{ProofCtx, ProofmanResult};
use sm_main::MainInstance;
use std::sync::Arc;
use zisk_common::{CheckPoint, Instance, InstanceCtx, InstanceType, Plan};

use crate::AirClassifier;
use crate::{state::ExecutionState, StaticSMBundle};

pub struct InstanceRegistry<F: PrimeField64> {
    /// State machine bundle for secondary instance creation.
    sm_bundle: Arc<StaticSMBundle<F>>,
}

impl<F: PrimeField64> InstanceRegistry<F> {
    /// Creates a new `InstanceRegistry`.
    ///
    /// # Arguments
    /// * `sm_bundle` - State machine bundle.
    pub fn new(sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self { sm_bundle }
    }

    /// Creates a main state machine instance.
    ///
    /// # Arguments
    /// * `plan` - The plan for the instance.
    /// * `global_id` - The global ID assigned to this instance.
    pub fn create_main_instance(&self, plan: Plan, global_id: usize) -> MainInstance<F> {
        MainInstance::new(InstanceCtx::new(global_id, plan), self.sm_bundle.get_std())
    }

    /// Creates a secondary state machine instance.
    ///
    /// # Arguments
    /// * `plan` - The plan for the instance.
    /// * `global_id` - The global ID assigned to this instance.
    pub fn create_secn_instance(&self, plan: Plan, global_id: usize) -> Box<dyn Instance<F>> {
        let ictx = InstanceCtx::new(global_id, plan);
        self.sm_bundle.build_instance(ictx)
    }

    /// Creates a secondary instance by looking up the plan in execution state.
    ///
    /// # Arguments
    /// * `state` - The execution state containing the plans.
    /// * `global_id` - The global ID to look up.
    pub fn create_secn_instance_from_state(
        &self,
        state: &ExecutionState<F>,
        global_id: usize,
    ) -> Box<dyn Instance<F>> {
        let mut secn_planning_guard = state.secn_planning.write().unwrap();

        // Find and remove in single operation using swap_remove for O(1) removal
        let plan = secn_planning_guard
            .iter()
            .position(|plan| plan.global_id == Some(global_id))
            .map(|idx| secn_planning_guard.swap_remove(idx))
            .unwrap_or_else(|| panic!("Secondary instance not found for global_id: {}", global_id));

        self.create_secn_instance(plan, global_id)
    }

    /// Populates main instances in the execution state.
    ///
    /// # Arguments
    /// * `state` - The execution state to populate.
    /// * `assignments` - Vector of (global_id, plan) pairs.
    pub fn populate_main_instances(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        assignments: Vec<(usize, Plan)>,
    ) -> ProofmanResult<()> {
        let mut main_instances = state.main_instances.write().unwrap();
        for (global_id, plan) in assignments {
            main_instances
                .entry(global_id)
                .or_insert_with(|| self.create_main_instance(plan, global_id));

            let is_mine = pctx.dctx_is_my_process_instance(global_id)?;
            if is_mine {
                pctx.set_witness_ready(global_id, false);
            }
        }
        Ok(())
    }

    /// Populates secondary instances in the execution state.
    ///
    /// # Arguments
    /// * `state` - The execution state to populate.
    /// * `global_ids` - Vector of global IDs for instances to create.
    pub fn populate_secn_instances(&self, state: &ExecutionState<F>, global_ids: &[usize]) {
        let mut secn_instances = state.secn_instances.write().unwrap();
        for &global_id in global_ids {
            secn_instances
                .entry(global_id)
                .or_insert_with(|| self.create_secn_instance_from_state(state, global_id));
        }
    }

    /// Gets a reference to the state machine bundle.
    pub fn sm_bundle(&self) -> &StaticSMBundle<F> {
        &self.sm_bundle
    }

    /// Configures secondary state machine instances based on planning.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `plannings` - Map of SM ID to plans.
    pub fn configure_sm_instances(
        &self,
        pctx: &ProofCtx<F>,
        plannings: &std::collections::BTreeMap<usize, Vec<Plan>>,
    ) {
        self.sm_bundle.configure_instances(pctx, plannings);
    }

    /// Configures checkpoints for secondary instances.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `state` - Execution state containing the instances.
    /// * `global_ids` - Global IDs of secondary instances to configure.
    pub fn configure_checkpoints(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) {
        let secn_instances = state.secn_instances.read().unwrap();

        for &global_id in global_ids {
            secn_instances[&global_id].reset();

            if secn_instances[&global_id].instance_type() == InstanceType::Instance {
                let checkpoint = secn_instances[&global_id].check_point();
                let chunks = match checkpoint {
                    CheckPoint::None => vec![],
                    CheckPoint::Single(chunk_id) => vec![chunk_id.as_usize()],
                    CheckPoint::Multiple(chunk_ids) => {
                        chunk_ids.iter().map(|id| id.as_usize()).collect()
                    }
                };

                let (_, air_id) =
                    pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info");
                let is_memory_related = AirClassifier::is_memory_related(air_id);
                pctx.dctx_set_chunks(global_id, chunks, is_memory_related);
            }
        }
    }
}
