//! Instance planning component.
//!
//! This module handles the planning and assignment of main and secondary
//! state machine instances to the proof context.

use fields::PrimeField64;
use proofman_common::{ProofCtx, SetupCtx};
use sm_main::MainPlanner;
use std::{collections::BTreeMap, sync::RwLock};
use zisk_common::{EmuTrace, InstanceType, Plan};
use zisk_pil::{MAIN_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::AirClassifier;
use crate::{DeviceMetricsList, NestedDeviceMetricsList, StaticSMBundle};

/// Output from main planning.
pub struct MainPlanningOutput {
    /// Plans for main instances.
    pub plans: Vec<Plan>,
    /// Public values extracted during planning.
    pub public_values: Vec<(u64, u32)>,
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
    /// * `main_count` - Device metrics for main instances.
    ///
    /// # Returns
    /// Planning output with plans and public values.
    pub fn plan_main<F: PrimeField64>(
        &self,
        min_traces: &[EmuTrace],
        main_count: DeviceMetricsList,
    ) -> MainPlanningOutput {
        let (plans, public_values) =
            MainPlanner::plan::<F>(min_traces, main_count, self.chunk_size);
        MainPlanningOutput { plans, public_values }
    }

    /// Plans secondary state machine instances.
    ///
    /// # Arguments
    /// * `sm_bundle` - State machine bundle.
    /// * `secn_count` - Device metrics for secondary instances.
    ///
    /// # Returns
    /// BTreeMap of SM type ID to plans.
    pub fn plan_secondary<F: PrimeField64>(
        &self,
        sm_bundle: &StaticSMBundle<F>,
        secn_count: &mut NestedDeviceMetricsList,
    ) -> BTreeMap<usize, Vec<Plan>> {
        sm_bundle.plan_sec(secn_count)
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
        sctx: &SetupCtx<F>,
        global_ids: &RwLock<Vec<usize>>,
        plans: Vec<Plan>,
    ) -> (Vec<(usize, Plan)>, u64) {
        let mut assignments = Vec::with_capacity(plans.len());

        let setup_main = sctx.get_setup(ZISK_AIRGROUP_ID, MAIN_AIR_IDS[0]).unwrap();
        let n_bits = setup_main.stark_info.stark_struct.n_bits;
        let total_cols: u64 = setup_main
            .stark_info
            .map_sections_n
            .iter()
            .filter(|(key, _)| *key != "const")
            .map(|(_, value)| *value)
            .sum();
        let cost = (1 << n_bits) * total_cols;
        let total_cost = cost * plans.len() as u64;

        for mut plan in plans {
            let global_id = pctx
                .add_instance_assign(plan.airgroup_id, plan.air_id)
                .expect("Failed to add instance");
            plan.set_global_id(global_id);
            global_ids.write().unwrap().push(global_id);
            assignments.push((global_id, plan));
        }

        (assignments, total_cost)
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
    ) {
        for plan in plans.iter_mut() {
            // ROM instances need special first partition assignment
            let global_id = if AirClassifier::is_rom_instance(plan.airgroup_id, plan.air_id) {
                pctx.add_instance_assign_first_process(plan.airgroup_id, plan.air_id)
                    .expect("Failed to add ROM instance")
            } else if AirClassifier::is_keccakf_instance(plan.airgroup_id, plan.air_id) {
                pctx.add_instance_assign(plan.airgroup_id, plan.air_id)
                    .expect("Failed to add KeccakF instance")
            } else {
                match plan.instance_type {
                    InstanceType::Instance => pctx
                        .add_instance(plan.airgroup_id, plan.air_id)
                        .expect("Failed to add instance"),
                    InstanceType::Table => {
                        pctx.add_table(plan.airgroup_id, plan.air_id).expect("Failed to add table")
                    }
                }
            };

            global_ids.write().unwrap().push(global_id);
            plan.set_global_id(global_id);
        }
    }
}
