//! Instance lifecycle component.
//!
//! After step 3.1, *construction* of main / secondary instances lives
//! in [`crate::InstanceFactory`]; this module retains the lifecycle
//! responsibilities (`populate_*`, `configure_sm_instances`,
//! `configure_checkpoints`) that interact with [`crate::ExecutionState`]
//! and [`proofman_common::ProofCtx`].
//!
//! The `swap_remove`-from-state lookup that used to live in
//! `create_secn_instance_from_state` is gone — callers now pass plans
//! directly to [`Self::populate_secn_instances`], removing the
//! producer/consumer round-trip through `ExecutionState::secn_planning`.

use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use zisk_common::{CheckPoint, InstanceType, Plan};

use crate::ports::{GlobalId, ProofRegistry};
use crate::AirClassifier;
use crate::{state::ExecutionState, InstanceFactory, StaticSMBundle};

use anyhow::Result;

/// Lifecycle owner for main + secondary instances on the executor.
pub struct InstanceRegistry<F: PrimeField64> {
    /// Factory used for actual instance construction. Holds the
    /// shared SM bundle.
    factory: InstanceFactory<F>,
}

impl<F: PrimeField64> InstanceRegistry<F> {
    /// Creates a new `InstanceRegistry` backed by `sm_bundle`.
    pub fn new(sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self { factory: InstanceFactory::new(sm_bundle) }
    }

    /// Populates main instances in the execution state.
    ///
    /// # Arguments
    /// * `registry` - Proof-context surface (used to gate
    ///   `set_witness_ready` on rank-owned mains).
    /// * `state` - Execution state to populate.
    /// * `assignments` - Vector of (global_id, plan) pairs.
    pub fn populate_main_instances(
        &self,
        registry: &dyn ProofRegistry,
        state: &ExecutionState<F>,
        assignments: Vec<(usize, Plan)>,
    ) -> Result<()> {
        let mut main_instances =
            state.instance_set.main_instances.write().map_err(|e| anyhow::anyhow!("{e}"))?;
        for (global_id, plan) in assignments {
            main_instances
                .entry(global_id)
                .or_insert_with(|| self.factory.new_main(plan, global_id));

            let gid = GlobalId(global_id);
            if registry.is_my_process_instance(gid)? {
                registry.set_witness_ready(gid, false);
            }
        }

        Ok(())
    }

    /// Populates secondary instances in the execution state.
    ///
    /// Each plan must already have its `global_id` stamped (done by
    /// `InstancePlanner::assign_secn_instances`). The plans are
    /// consumed in place: this method moves each plan into the
    /// factory call rather than re-reading from state — no
    /// `swap_remove`, no `RwLock<Vec<Plan>>` round-trip.
    pub fn populate_secn_instances(
        &self,
        state: &ExecutionState<F>,
        plans: Vec<Plan>,
    ) -> Result<()> {
        let mut secn_instances =
            state.instance_set.secn_instances.write().map_err(|e| anyhow::anyhow!("{e}"))?;
        for plan in plans {
            let global_id = plan
                .global_id
                .ok_or_else(|| anyhow::anyhow!("secn plan missing global_id before populate"))?;
            if let Entry::Vacant(e) = secn_instances.entry(global_id) {
                let instance = self.factory.new_secn(plan, global_id)?;
                e.insert(instance);
            }
        }

        Ok(())
    }

    /// Gets a reference to the state machine bundle.
    pub fn sm_bundle(&self) -> &StaticSMBundle<F> {
        self.factory.sm_bundle()
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
        self.factory.sm_bundle().configure_instances(pctx, plannings);
    }

    /// Configures checkpoints for secondary instances.
    ///
    /// # Arguments
    /// * `registry` - Proof-context surface for instance-info lookup
    ///   and chunk assignment.
    /// * `state` - Execution state containing the instances.
    /// * `global_ids` - Global IDs of secondary instances to configure.
    pub fn configure_checkpoints(
        &self,
        registry: &dyn ProofRegistry,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) -> Result<()> {
        let secn_instances =
            state.instance_set.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;

        for &global_id in global_ids {
            let instance = secn_instances
                .get(&global_id)
                .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

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
