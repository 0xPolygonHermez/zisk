//! Instance planning component.
//!
//! This module handles the planning and assignment of main and secondary
//! state machine instances to the proof context.

use fields::PrimeField64;
use proofman_common::ProofCtx;
use sm_main::{MainPlanner, MainSmError};
use std::{collections::BTreeMap, sync::RwLock};
use zisk_common::{EmuTrace, InstanceType, Plan};
use zisk_pil::{ROM_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::AirClassifier;
use crate::{CountersChunkMetrics, StaticSMBundle};

use anyhow::Result;

/// Output from main planning.
pub struct MainPlanningOutput {
    /// Plans for main instances.
    pub plans: Vec<Plan>,
}

/// Component responsible for instance planning.
///
/// Handles the strategic planning of main and secondary state machine
/// instances based on execution metrics. Planning determines:
/// - How many instances of each state machine type are needed
/// - How work is distributed across instances
/// - Global ID assignments for proof context registration
pub struct InstancePlanner {
    /// Chunk size for dividing execution into manageable pieces.
    chunk_size: u64,
}

impl InstancePlanner {
    /// Creates a new `InstancePlanner`.
    ///
    /// # Arguments
    /// * `chunk_size` - The chunk size for processing.
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size }
    }

    /// Plans main state machine instances.
    ///
    /// # Arguments
    /// * `min_traces` - Minimal traces from execution.
    ///
    /// # Returns
    /// Planning output containing the main-instance plans, or a `MainSmError`
    /// if the planner rejects the configuration.
    pub fn plan_main(
        &self,
        min_traces: &[EmuTrace],
    ) -> std::result::Result<MainPlanningOutput, MainSmError> {
        let plans = MainPlanner::plan(min_traces, self.chunk_size)?;
        Ok(MainPlanningOutput { plans })
    }

    /// Plans secondary state machine instances.
    ///
    /// # Arguments
    /// * `sm_bundle` - State machine bundle.
    /// * `counters` - Device metrics for secondary instances.
    ///
    /// # Returns
    /// BTreeMap of SM type ID to plans.
    pub fn plan_secondary<F: PrimeField64>(
        &self,
        sm_bundle: &StaticSMBundle<F>,
        counters: &mut CountersChunkMetrics,
        is_asm_emulator: bool,
    ) -> BTreeMap<usize, Vec<Plan>> {
        sm_bundle.plan_sec(counters, is_asm_emulator)
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
            } else if AirClassifier::is_keccakf_instance(plan.airgroup_id, plan.air_id) {
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
