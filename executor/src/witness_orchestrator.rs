//! Witness orchestrator component.
//!
//! This module handles the logic for witness computation, coordinating between collectors and
//! witness generators

use crate::{
    state::ExecutionState, AirClassifier, ChunkDataCollector, StaticSMBundle, WitnessGenerator,
};
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, ProofmanResult, SetupCtx};
use sm_rom::RomInstance;
use std::collections::HashMap;
use std::sync::Arc;
use zisk_common::{BusDevice, Instance, InstanceType, Stats, StatsScope};
use zisk_core::ZiskRom;

/// Type alias for the secondary instances map (owned).
type SecnInstanceMap<F> = HashMap<usize, Box<dyn Instance<F>>>;

/// Type alias for the secondary instances map (borrowed).
type SecnInstanceMapRef<'a, F> = HashMap<usize, &'a Box<dyn Instance<F>>>;

/// Context for witness computation operations.
pub struct WitnessContext<'a, F: PrimeField64> {
    /// Proof context.
    pub pctx: &'a ProofCtx<F>,

    /// Setup context.
    pub sctx: &'a SetupCtx<F>,

    /// Execution state.
    pub state: &'a ExecutionState<F>,

    /// Buffer pool for trace data.
    pub buffer_pool: &'a dyn BufferPool<F>,

    /// Statistics scope.
    pub stats_scope: &'a StatsScope,
}

impl<'a, F: PrimeField64> WitnessContext<'a, F> {
    /// Creates a new witness context.
    pub fn new(
        pctx: &'a ProofCtx<F>,
        sctx: &'a SetupCtx<F>,
        state: &'a ExecutionState<F>,
        buffer_pool: &'a dyn BufferPool<F>,
        stats_scope: &'a StatsScope,
    ) -> Self {
        Self { pctx, sctx, state, buffer_pool, stats_scope }
    }

    /// Gets instance info (airgroup_id, air_id) for a global ID.
    pub fn get_instance_info(&self, global_id: usize) -> (usize, usize) {
        self.pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info")
    }
}

/// Component responsible for orchestrating witness computation.
pub struct WitnessOrchestrator<F: PrimeField64> {
    /// Chunk data collector for secondary instances.
    collector: ChunkDataCollector<F>,

    /// Witness computer for all instance types.
    witness_generator: WitnessGenerator,

    /// Whether using ASM emulator (cached to avoid passing through all calls).
    is_asm_emulator: bool,
}

impl<F: PrimeField64> WitnessOrchestrator<F> {
    /// Creates a new `WitnessOrchestrator`.
    ///
    /// # Arguments
    /// * `chunk_size` - Chunk size for trace processing.
    /// * `sm_bundle` - Static state machine bundle for collector initialization.
    /// * `is_asm_emulator` - Whether using ASM emulator.
    pub fn new(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>, is_asm_emulator: bool) -> Self {
        let collector = ChunkDataCollector::new(sm_bundle.clone());
        let witness_generator = WitnessGenerator::new(chunk_size);

        Self { collector, witness_generator, is_asm_emulator }
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) {
        self.collector.set_rom(zisk_rom.clone());
    }

    /// Computes witness for a single global ID.
    ///
    /// Routes to the appropriate witness computation method based on
    /// instance type and handles special cases like ROM with ASM emulator.
    ///
    /// # Arguments
    /// * `ctx` - Witness context with shared references.
    /// * `global_id` - Global ID of the instance.
    pub fn compute_witness_for_instance(
        &self,
        ctx: &WitnessContext<'_, F>,
        global_id: usize,
    ) -> ProofmanResult<()> {
        let (airgroup_id, air_id) = ctx.get_instance_info(global_id);

        if AirClassifier::is_main(air_id) {
            self.compute_main_witness(
                ctx.pctx,
                ctx.state,
                global_id,
                ctx.buffer_pool,
                ctx.stats_scope,
            )
        } else {
            self.compute_secondary_witness(
                ctx.pctx,
                ctx.sctx,
                ctx.state,
                global_id,
                airgroup_id,
                air_id,
                ctx.buffer_pool,
                ctx.stats_scope,
            )
        }
    }

    /// Computes witness for a main instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `state` - Execution state.
    /// * `global_id` - Global ID of the main instance.
    /// * `buffer_pool` - Buffer pool for trace data.
    /// * `stats_scope` - Statistics scope for recording stats.
    fn compute_main_witness(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        global_id: usize,
        buffer_pool: &dyn BufferPool<F>,
        stats_scope: &StatsScope,
    ) -> ProofmanResult<()> {
        let main_instances = state.main_instances.read().unwrap();
        let main_instance = &main_instances[&global_id];

        self.witness_generator.compute_main_witness(
            pctx,
            state,
            main_instance,
            buffer_pool.take_buffer(),
            stats_scope.id(),
        )
    }

    /// Computes witness for a secondary instance.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `sctx` - Setup context.
    /// * `state` - Execution state.
    /// * `global_id` - Global ID of the secondary instance.
    /// * `airgroup_id` - AIR group ID of the instance.
    /// * `air_id` - AIR ID of the instance.
    /// * `buffer_pool` - Buffer pool for trace data.
    /// * `stats_scope` - Statistics scope for recording stats.
    #[allow(clippy::too_many_arguments)]
    fn compute_secondary_witness(
        &self,
        pctx: &ProofCtx<F>,
        sctx: &SetupCtx<F>,
        state: &ExecutionState<F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
        buffer_pool: &dyn BufferPool<F>,
        stats_scope: &StatsScope,
    ) -> ProofmanResult<()> {
        let secn_instances = state.secn_instances.read().unwrap();
        let secn_instance = &secn_instances[&global_id];

        if secn_instance.instance_type() == InstanceType::Instance {
            let needs_collection =
                !state.collectors_by_instance.read().unwrap().contains_key(&global_id);

            if needs_collection {
                if AirClassifier::is_rom(air_id) && self.is_asm_emulator {
                    // ROM with ASM emulator: skip collection
                    self.register_empty_collector(state, global_id, airgroup_id, air_id);
                } else {
                    // Collect data for this instance
                    self.collector.collect_single(pctx, state, global_id, secn_instance).map_err(
                        |e| proofman_common::ProofmanError::InvalidConfiguration(e.to_string()),
                    )?;
                }
            }
        }

        let instance = &**secn_instance;
        let collectors =
            Self::take_collectors_for_instance(state, global_id, instance.instance_type());

        self.witness_generator.compute_secn_witness(
            pctx,
            sctx,
            state,
            global_id,
            instance,
            collectors,
            buffer_pool.take_buffer(),
            stats_scope.id(),
        )
    }

    /// Registers an empty collector for instances that skip collection.
    ///
    /// # Arguments
    /// * `state` - Execution state.
    /// * `global_id` - Global ID of the instance.
    /// * `airgroup_id` - AIR group ID of the instance.
    /// * `air_id` - AIR ID of the instance.
    fn register_empty_collector(
        &self,
        state: &ExecutionState<F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) {
        let stats = Stats::new_no_collection(airgroup_id, air_id);

        state.collectors_by_instance.write().unwrap().insert(global_id, Vec::new());
        state.stats.insert_witness_stats(global_id, stats);
    }

    /// Extracts collectors from state, returning an empty list for table instances.
    ///
    /// # Arguments
    /// * `state` - Execution state.
    /// * `global_id` - Global ID of the instance.
    /// * `instance_type` - Type of the instance (Instance or Table).
    fn take_collectors_for_instance(
        state: &ExecutionState<F>,
        global_id: usize,
        instance_type: InstanceType,
    ) -> Vec<(usize, Box<dyn BusDevice<u64>>)> {
        match instance_type {
            InstanceType::Instance => {
                let mut guard = state.collectors_by_instance.write().unwrap();

                guard
                    .remove(&global_id)
                    .expect("Missing collectors for given global_id")
                    .into_iter()
                    .enumerate()
                    .map(|(idx, opt)| {
                        opt.unwrap_or_else(|| {
                            panic!("Collector at index {} for global_id {} is None", idx, global_id)
                        })
                    })
                    .collect()
            }
            InstanceType::Table => {
                vec![]
            }
        }
    }

    /// Pre-calculates witnesses by determining which instances need collection.
    ///
    /// Sets witness readiness flags and collects data for instances that need it.
    ///
    /// # Arguments
    /// * `pctx` - Proof context.
    /// * `state` - Execution state.
    /// * `global_ids` - Global IDs to pre-calculate.
    pub fn pre_calculate(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) -> ProofmanResult<()> {
        let secn_instances_guard = state.secn_instances.read().unwrap();

        let mut instances_to_collect = HashMap::new();

        for &global_id in global_ids {
            let (airgroup_id, air_id) =
                pctx.dctx_get_instance_info(global_id).expect("Failed to get instance info");

            if AirClassifier::is_main(air_id) {
                pctx.set_witness_ready(global_id, false);
            } else if AirClassifier::is_rom(air_id) {
                self.handle_rom_pre_calculate(
                    pctx,
                    state,
                    &secn_instances_guard,
                    &mut instances_to_collect,
                    global_id,
                    airgroup_id,
                    air_id,
                );
            } else {
                self.handle_secondary_pre_calculate(
                    pctx,
                    state,
                    &secn_instances_guard,
                    &mut instances_to_collect,
                    global_id,
                );
            }
        }

        // Collect all instances that need collection
        if !instances_to_collect.is_empty() {
            self.collector
                .collect(pctx, state, instances_to_collect)
                .map_err(|e| proofman_common::ProofmanError::InvalidConfiguration(e.to_string()))?;
        }
        Ok(())
    }

    /// Handles ROM instance pre-calculation.
    #[allow(clippy::too_many_arguments)]
    fn handle_rom_pre_calculate<'a>(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) {
        if self.is_asm_emulator {
            pctx.set_witness_ready(global_id, false);
        } else {
            let secn_instance = &secn_instances[&global_id];
            let rom_instance = secn_instance.as_any().downcast_ref::<RomInstance>().unwrap();

            if rom_instance.skip_collector() {
                self.register_empty_collector(state, global_id, airgroup_id, air_id);
                pctx.set_witness_ready(global_id, true);
            } else {
                instances_to_collect.insert(global_id, secn_instance);
            }
        }
    }

    /// Handles secondary instance pre-calculation.
    fn handle_secondary_pre_calculate<'a>(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
    ) {
        let secn_instance = &secn_instances[&global_id];

        if secn_instance.instance_type() == InstanceType::Instance
            && !state.collectors_by_instance.read().unwrap().contains_key(&global_id)
        {
            instances_to_collect.insert(global_id, secn_instance);
        } else {
            pctx.set_witness_ready(global_id, true);
        }
    }
}
