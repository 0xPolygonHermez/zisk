//! Witness computation component.
//!
//! This module handles the computation of witnesses for main and
//! secondary state machine instances.

use fields::PrimeField64;
use proofman_common::{ProofCtx, ProofmanResult, SetupCtx};
use sm_main::MainInstance;
use std::time::Instant;
use zisk_common::{stats_begin, stats_end, BusDevice, Instance, InstanceType, Stats};

use crate::state::ExecutionState;

/// Component responsible for witness computation.
///
/// Handles the computation of witnesses for:
/// - **Main instances**: Compute from minimal traces with chunk processing
/// - **Secondary instances**: Compute from collected chunk data
/// - **Table instances**: Compute static lookup tables
pub struct WitnessGenerator {
    /// Chunk size for trace processing.
    chunk_size: u64,
}

impl WitnessGenerator {
    /// Creates a new `WitnessGenerator`.
    ///
    /// # Arguments
    /// * `chunk_size` - Chunk size for processing.
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size }
    }

    /// Computes witness for a main state machine instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `state` - Execution state.
    /// * `main_instance` - The main instance to compute witness for.
    /// * `trace_buffer` - Buffer for trace data.
    /// * `caller_stats_id` - Parent stats scope ID.
    pub fn compute_main_witness<F: PrimeField64>(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        main_instance: &MainInstance<F>,
        trace_buffer: Vec<F>,
        _caller_stats_id: u64,
    ) -> ProofmanResult<()> {
        let witness_start_time = Instant::now();

        let (airgroup_id, air_id) = pctx
            .dctx_get_instance_info(main_instance.ictx.global_id)
            .expect("Failed to get instance info");

        stats_begin!(state.stats, _caller_stats_id, _stats_scope, "AIR_MAIN_WITNESS", air_id);

        let zisk_rom = state
            .get_rom()
            .map_err(|e| proofman_common::ProofmanError::InvalidConfiguration(e.to_string()))?;
        let min_traces_guard = state.min_traces.read().unwrap();
        let min_traces = min_traces_guard.as_ref().expect("min_traces should not be None");

        let air_instance = main_instance.compute_witness(
            &zisk_rom,
            min_traces,
            self.chunk_size,
            main_instance,
            trace_buffer,
        )?;

        pctx.add_air_instance(air_instance, main_instance.ictx.global_id);

        stats_end!(state.stats, &_stats_scope);

        let stats = Stats::new_main_completed(airgroup_id, air_id, witness_start_time);

        state.stats.insert_witness_stats(main_instance.ictx.global_id, stats);

        Ok(())
    }

    /// Computes witness for a secondary state machine instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `state` - Execution state.
    /// * `global_id` - Global ID of the instance.
    /// * `secn_instance` - The secondary instance to compute witness for.
    /// * `collectors` - Collectors for chunk data.
    /// * `should_add_instance` - Whether to add the computed AIR instance to the proof
    /// * `trace_buffer` - Buffer for trace data.
    /// * `_caller_stats_id` - Parent stats scope ID.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_secn_witness<F: PrimeField64>(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        state: &ExecutionState<F>,
        global_id: usize,
        secn_instance: &dyn Instance<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<u64>>)>,
        trace_buffer: Vec<F>,
        _caller_stats_id: u64,
    ) -> ProofmanResult<()> {
        let witness_start_time = Instant::now();

        let _stats_msg = match secn_instance.instance_type() {
            InstanceType::Instance => "AIR_SECN_WITNESS",
            InstanceType::Table => "AIR_WITNESS_TABLE",
        };

        let (_airgroup_id, _air_id) =
            pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info");

        stats_begin!(state.stats, _caller_stats_id, _stats_scope, _stats_msg, _air_id);

        let air_instance = secn_instance.compute_witness(pctx, sctx, collectors, trace_buffer)?;

        if let Some(air_instance) = air_instance {
            let should_add_instance = secn_instance.instance_type() == InstanceType::Instance
                || (secn_instance.instance_type() == InstanceType::Table
                    && pctx
                        .dctx_is_my_process_instance(global_id)
                        .expect("Failed to check instance ownership"));

            if should_add_instance {
                pctx.add_air_instance(air_instance, global_id);
            }
        }

        stats_end!(state.stats, &_stats_scope);

        state.stats.set_witness_duration(global_id, witness_start_time.elapsed().as_millis());

        Ok(())
    }
}
