//! Instance assignment helpers — the `pctx`-mutating half of the old
//! `InstancePlanner` API.
//!
//! Step 3.3: every `pctx.…` call has been routed through
//! [`crate::ProofRegistry`] (the executor's anti-corruption layer over
//! `ProofCtx<F>`). `InstancePlanner` is now field-erased over `F` and
//! mockable in unit tests via [`crate::ports::fakes::FakeProofRegistry`].

use anyhow::Result;
use std::sync::RwLock;
use zisk_common::{InstanceType, Plan};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::ports::{GlobalId, InstanceInfo, ProofRegistry};
use crate::AirClassifier;

/// Assigner of state-machine instances to the proof context.
///
/// Stateless and `F`-free — every method takes a
/// `&dyn ProofRegistry`, so the assigner can be exercised in unit
/// tests against [`crate::ports::fakes::FakeProofRegistry`] without
/// any `ProofCtx<F>` setup.
pub struct InstancePlanner;

impl InstancePlanner {
    /// Creates a new `InstancePlanner`.
    pub fn new() -> Self {
        Self
    }

    /// Assigns the ROM instance to the proof context.
    ///
    /// Returns the assigned [`GlobalId`].
    pub fn assign_rom_instance(&self, registry: &dyn ProofRegistry) -> Result<GlobalId> {
        registry.add_instance_assign(InstanceInfo::new(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]))
    }

    /// Assigns main instances to the proof context.
    ///
    /// # Arguments
    /// * `registry` - Proof-context assignment surface.
    /// * `global_ids` - Lock for storing assigned global IDs.
    /// * `plans` - Plans to assign.
    ///
    /// # Returns
    /// Vector of (global_id, plan) pairs for instance creation.
    pub fn assign_main_instances(
        &self,
        registry: &dyn ProofRegistry,
        global_ids: &RwLock<Vec<usize>>,
        plans: Vec<Plan>,
    ) -> Result<Vec<(usize, Plan)>> {
        let mut assignments = Vec::with_capacity(plans.len());

        for mut plan in plans {
            let gid = registry.add_instance_assign(InstanceInfo::new(plan.airgroup_id, plan.air_id))?;
            plan.set_global_id(gid.0);
            global_ids.write().map_err(|e| anyhow::anyhow!("{e}"))?.push(gid.0);
            assignments.push((gid.0, plan));
        }

        Ok(assignments)
    }

    /// Assigns secondary instances to the proof context.
    ///
    /// # Arguments
    /// * `registry` - Proof-context assignment surface.
    /// * `global_ids` - Lock for storing assigned global IDs.
    /// * `plans` - Plans to assign (will be mutated with global IDs).
    pub fn assign_secn_instances(
        &self,
        registry: &dyn ProofRegistry,
        global_ids: &RwLock<Vec<usize>>,
        plans: &mut [Plan],
    ) -> Result<()> {
        for plan in plans.iter_mut() {
            let info = InstanceInfo::new(plan.airgroup_id, plan.air_id);

            // ROM instances need special first-partition assignment —
            // look up the global id stamped by `assign_rom_instance`
            // earlier in the phase.
            let gid = if AirClassifier::is_rom_instance(plan.airgroup_id, plan.air_id) {
                registry.find_instance_id(InstanceInfo::new(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]))?
            } else if AirClassifier::is_rank_assigned_precompile_instance(
                plan.airgroup_id,
                plan.air_id,
            ) {
                registry.add_instance_assign(info)?
            } else {
                match plan.instance_type {
                    InstanceType::Instance => registry.add_instance(info)?,
                    InstanceType::Table => registry.add_table(info)?,
                }
            };

            global_ids.write().map_err(|e| anyhow::anyhow!("{e}"))?.push(gid.0);
            plan.set_global_id(gid.0);
        }

        Ok(())
    }
}

impl Default for InstancePlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::fakes::{AddKind, FakeProofRegistry};

    #[test]
    fn assign_rom_instance_uses_add_instance_assign_with_rom_air_id() {
        let registry = FakeProofRegistry::new();
        let planner = InstancePlanner::new();

        let gid = planner.assign_rom_instance(&registry).expect("ok");

        let calls = registry.additions.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].kind, AddKind::InstanceAssign);
        assert_eq!(calls[0].info, InstanceInfo::new(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]));
        assert_eq!(calls[0].gid, gid);
    }

    #[test]
    fn assign_main_instances_records_one_add_instance_assign_per_plan() {
        let registry = FakeProofRegistry::new();
        let planner = InstancePlanner::new();
        let global_ids = RwLock::new(Vec::<usize>::new());

        // Two synthetic plans pointing at arbitrary (group, air) tuples.
        // Set instance_type to Instance — the field is unused on the
        // main-assign path but must satisfy `Plan::new`.
        let plans = vec![
            Plan::new(7, 100, None, InstanceType::Instance, zisk_common::CheckPoint::None, None),
            Plan::new(7, 101, None, InstanceType::Instance, zisk_common::CheckPoint::None, None),
        ];

        let assignments = planner
            .assign_main_instances(&registry, &global_ids, plans)
            .expect("ok");

        assert_eq!(assignments.len(), 2);
        let calls = registry.additions.borrow();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|c| c.kind == AddKind::InstanceAssign));
        assert_eq!(calls[0].info, InstanceInfo::new(7, 100));
        assert_eq!(calls[1].info, InstanceInfo::new(7, 101));

        // global_ids slot must receive both assignments in order.
        let gids = global_ids.read().unwrap();
        assert_eq!(gids.len(), 2);
        assert_eq!(gids[0], calls[0].gid.0);
        assert_eq!(gids[1], calls[1].gid.0);

        // Each plan must carry its assigned global_id.
        assert_eq!(assignments[0].1.global_id, Some(calls[0].gid.0));
        assert_eq!(assignments[1].1.global_id, Some(calls[1].gid.0));
    }

    #[test]
    fn assign_secn_instances_routes_by_instance_type() {
        let registry = FakeProofRegistry::new();
        let planner = InstancePlanner::new();
        let global_ids = RwLock::new(Vec::<usize>::new());

        // Plain Instance → add_instance, Table → add_table.
        let mut plans = vec![
            Plan::new(7, 200, None, InstanceType::Instance, zisk_common::CheckPoint::None, None),
            Plan::new(7, 201, None, InstanceType::Table, zisk_common::CheckPoint::None, None),
        ];

        planner.assign_secn_instances(&registry, &global_ids, &mut plans).expect("ok");

        let calls = registry.additions.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].kind, AddKind::Instance);
        assert_eq!(calls[1].kind, AddKind::Table);
    }
}
