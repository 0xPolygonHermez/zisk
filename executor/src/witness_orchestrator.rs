//! Witness orchestrator component.
//!
//! This module handles the logic for witness computation, coordinating between collectors and
//! witness generators

use crate::AsmRunnerRH;
use crate::{
    state::ExecutionState, AirClassifier, ChunkDataCollector, StaticSMBundle, WitnessGenerator,
};
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};
use sm_rom::RomInstance;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zisk_common::{BusDevice, Instance, InstanceType, Stats, StatsScope};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

use anyhow::Result;

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

    pub is_asm_emulator: bool,
}

impl<'a, F: PrimeField64> WitnessContext<'a, F> {
    /// Creates a new witness context.
    pub fn new(
        pctx: &'a ProofCtx<F>,
        sctx: &'a SetupCtx<F>,
        state: &'a ExecutionState<F>,
        buffer_pool: &'a dyn BufferPool<F>,
        stats_scope: &'a StatsScope,
        is_asm_emulator: bool,
    ) -> Self {
        Self { pctx, sctx, state, buffer_pool, stats_scope, is_asm_emulator }
    }

    /// Gets instance info (airgroup_id, air_id) for a global ID.
    pub fn get_instance_info(&self, global_id: usize) -> Result<(usize, usize)> {
        Ok(self.pctx.dctx_get_instance_info(global_id)?)
    }
}

/// Component responsible for orchestrating witness computation.
pub struct WitnessOrchestrator<F: PrimeField64> {
    /// Chunk data collector for secondary instances.
    collector: ChunkDataCollector<F>,

    /// Witness computer for all instance types.
    witness_generator: WitnessGenerator,

    trace_buffer_rom: Mutex<Vec<F>>,
}

impl<F: PrimeField64> WitnessOrchestrator<F> {
    /// Creates a new `WitnessOrchestrator`.
    ///
    /// # Arguments
    /// * `chunk_size` - Chunk size for trace processing.
    /// * `sm_bundle` - Static state machine bundle for collector initialization.
    pub fn new(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        let collector = ChunkDataCollector::new(sm_bundle.clone());
        let witness_generator = WitnessGenerator::new(chunk_size);
        let trace_buffer_rom = Mutex::new(vec![F::ZERO; RomTrace::<F>::NUM_ROWS]);
        Self { collector, witness_generator, trace_buffer_rom }
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) -> Result<()> {
        self.collector.set_rh_data(rh_data)
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> Result<()> {
        self.collector.set_rom(zisk_rom.clone())
    }

    pub fn set_packed(&self, packed: bool) {
        self.witness_generator.set_packed(packed);
    }
    
    pub fn reset(&self) -> Result<()> {
        *self.trace_buffer_rom.lock().map_err(|e| anyhow::anyhow!("{e}"))? =
            vec![F::ZERO; RomTrace::<F>::NUM_ROWS];

        Ok(())
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
    ) -> Result<()> {
        let (airgroup_id, air_id) = ctx.get_instance_info(global_id)?;

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
                ctx.is_asm_emulator,
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
    ) -> Result<()> {
        let main_instances = state.main_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
        let main_instance = main_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

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
        is_asm_emulator: bool,
    ) -> Result<()> {
        let secn_instances = state.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
        let secn_instance = secn_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

        if secn_instance.instance_type() == InstanceType::Instance {
            let needs_collection = !state
                .collectors_by_instance
                .read()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .contains_key(&global_id);

            if needs_collection {
                if AirClassifier::is_rom(air_id) && is_asm_emulator {
                    // ROM with ASM emulator: skip collection
                    self.register_empty_collector(state, global_id, airgroup_id, air_id)?;
                } else {
                    // Collect data for this instance
                    self.collector
                        .collect_single(pctx, state, global_id, secn_instance)
                        .map_err(|e| anyhow::anyhow!("Collector error: {e}"))?;
                }
            }
        }

        let instance = &**secn_instance;
        let collectors =
            Self::take_collectors_for_instance(state, global_id, instance.instance_type())?;

        let trace_buffer = match AirClassifier::is_rom(air_id) {
            true => std::mem::take(
                &mut *self.trace_buffer_rom.lock().map_err(|e| anyhow::anyhow!("{e}"))?,
            ),
            false => buffer_pool.take_buffer(),
        };

        self.witness_generator.compute_secn_witness(
            pctx,
            sctx,
            state,
            global_id,
            instance,
            collectors,
            trace_buffer,
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
    ) -> Result<()> {
        let stats = Stats::new_no_collection(airgroup_id, air_id);

        state
            .collectors_by_instance
            .write()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .insert(global_id, Vec::new());
        state.stats.insert_witness_stats(global_id, stats);

        Ok(())
    }

    /// Extracts collectors from state, returning an empty list for table instances.
    ///
    /// # Arguments
    /// * `state` - Execution state.
    /// * `global_id` - Global ID of the instance.
    /// * `instance_type` - Type of the instance (Instance or Table).
    #[allow(clippy::type_complexity)]
    fn take_collectors_for_instance(
        state: &ExecutionState<F>,
        global_id: usize,
        instance_type: InstanceType,
    ) -> Result<Vec<(usize, Box<dyn BusDevice<u64>>)>> {
        match instance_type {
            InstanceType::Instance => {
                let mut guard =
                    state.collectors_by_instance.write().map_err(|e| anyhow::anyhow!("{e}"))?;

                let collectors = guard.remove(&global_id).ok_or_else(|| {
                    anyhow::anyhow!("Missing collectors for global_id {global_id}")
                })?;

                let result = collectors
                    .into_iter()
                    .enumerate()
                    .map(|(idx, opt)| {
                        opt.ok_or_else(|| {
                            anyhow::anyhow!(
                                "Collector at index {idx} for global_id {global_id} is None"
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(result)
            }
            InstanceType::Table => Ok(vec![]),
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
        is_asm_emulator: bool,
    ) -> Result<()> {
        let secn_instances_guard =
            state.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut instances_to_collect = HashMap::new();

        for &global_id in global_ids {
            let (airgroup_id, air_id) = pctx.dctx_get_instance_info(global_id)?;

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
                    is_asm_emulator,
                )?;
            } else {
                self.handle_secondary_pre_calculate(
                    pctx,
                    state,
                    &secn_instances_guard,
                    &mut instances_to_collect,
                    global_id,
                )?;
            }
        }

        // Collect all instances that need collection
        if !instances_to_collect.is_empty() {
            self.collector
                .collect(pctx, state, instances_to_collect)
                .map_err(|e| anyhow::anyhow!("Collector error: {e}"))?;
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
        is_asm_emulator: bool,
    ) -> Result<()> {
        if is_asm_emulator {
            pctx.set_witness_ready(global_id, false);
        } else {
            let secn_instance = secn_instances
                .get(&global_id)
                .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;
            let rom_instance =
                secn_instance.as_any().downcast_ref::<RomInstance>().ok_or_else(|| {
                    anyhow::anyhow!("Downcast failed: instance {global_id} to RomInstance")
                })?;

            if rom_instance.skip_collector() {
                self.register_empty_collector(state, global_id, airgroup_id, air_id)?;
                pctx.set_witness_ready(global_id, true);
            } else {
                instances_to_collect.insert(global_id, secn_instance);
            }
        }

        Ok(())
    }

    /// Handles secondary instance pre-calculation.
    fn handle_secondary_pre_calculate<'a>(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
    ) -> Result<()> {
        let secn_instance = secn_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

        if secn_instance.instance_type() == InstanceType::Instance
            && !state
                .collectors_by_instance
                .read()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .contains_key(&global_id)
        {
            instances_to_collect.insert(global_id, secn_instance);
        } else {
            pctx.set_witness_ready(global_id, true);
        }

        Ok(())
    }
}
