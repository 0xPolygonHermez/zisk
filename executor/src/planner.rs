//! Instance-to-`ProofCtx` assignment helpers.
//!
//! After step 2.5, this module is the **assignment-only** half of the
//! old `InstancePlanner` — the pure planning surface (`plan_main`,
//! `plan_secondary`) has moved to [`crate::PlanPhase`]. What stays
//! here is exclusively `pctx`-mutating: registering instances with the
//! proof context and stamping global IDs back onto the plans.
//!
//! Future steps relocate these into `MaterializePhase`; for now the
//! struct keeps its name to minimise call-site churn at the executor.

use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::sync::RwLock;
use zisk_common::{InstanceType, Plan};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::AirClassifier;

use anyhow::Result;

/// Assigner of state-machine instances to the proof context.
///
/// Stateless — the field-free struct is kept to preserve today's
/// `self.planner.assign_*` call sites until M3 folds these into
/// `MaterializePhase`.
pub struct InstancePlanner;

impl InstancePlanner {
    /// Creates a new `InstancePlanner`.
    pub fn new() -> Self {
        Self
    }

    /// Assigns ROM instance to the proof context.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    ///
    /// # Returns
    /// Global ID assigned to the ROM instance.
    pub fn assign_rom_instance<F: PrimeField64>(&self, pctx: &ProofCtx<F>) -> Result<usize> {
        Ok(pctx.add_instance_assign(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])?)
    }

    /// Assigns main instances to the proof context.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `global_ids` - Lock for storing assigned global IDs.
    /// * `plans` - Plans to assign.
    ///
    /// # Returns
    /// Vector of (global_id, plan) pairs for instance creation.
    pub fn assign_main_instances<F: PrimeField64>(
        &self,
        pctx: &ProofCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        plans: Vec<Plan>,
    ) -> Result<Vec<(usize, Plan)>> {
        let mut assignments = Vec::with_capacity(plans.len());

        for mut plan in plans {
            let global_id = pctx.add_instance_assign(plan.airgroup_id, plan.air_id)?;
            plan.set_global_id(global_id);
            global_ids.write().map_err(|e| anyhow::anyhow!("{e}"))?.push(global_id);
            assignments.push((global_id, plan));
        }

        Ok(assignments)
    }

    /// Assigns secondary instances to the proof context.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `global_ids` - Lock for storing assigned global IDs.
    /// * `plans` - Plans to assign (will be mutated with global IDs).
    pub fn assign_secn_instances<F: PrimeField64>(
        &self,
        pctx: &ProofCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        plans: &mut [Plan],
    ) -> Result<()> {
        for plan in plans.iter_mut() {
            // ROM instances need special first partition assignment
            let global_id = if AirClassifier::is_rom_instance(plan.airgroup_id, plan.air_id) {
                let (_, id) = pctx.dctx_find_instance_id(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0])?;
                id
            } else if AirClassifier::is_rank_assigned_precompile_instance(
                plan.airgroup_id,
                plan.air_id,
            ) {
                pctx.add_instance_assign(plan.airgroup_id, plan.air_id)?
            } else {
                match plan.instance_type {
                    InstanceType::Instance => pctx.add_instance(plan.airgroup_id, plan.air_id)?,
                    InstanceType::Table => pctx.add_table(plan.airgroup_id, plan.air_id)?,
                }
            };

            global_ids.write().map_err(|e| anyhow::anyhow!("{e}"))?.push(global_id);
            plan.set_global_id(global_id);
        }

        Ok(())
    }
}

impl Default for InstancePlanner {
    fn default() -> Self {
        Self::new()
    }
}
