//! [`WitnessRouter`] — phase-4 actor that dispatches witness
//! computation per global id.
//!
//! Replaces the old `WitnessOrchestrator`. The substantive
//! behaviour is identical, but:
//!   * `is_asm_emulator` is **baked at construction** via
//!     [`Self::new_asm`] / [`Self::new_native`] instead of being passed
//!     per call. The witness-side never branches on backend at
//!     dispatch time — the ROM path is the right one for the bundle
//!     this router was built for.
//!   * The public entry-point is now [`Self::dispatch`].
//!
//! Step 4.2 will replace the `&ProofCtx<F>` borrows that still flow
//! through here with `&dyn ProofRegistry` + `&dyn SetupAccess`, at
//! which point the per-category handlers become unit-testable. For
//! now this is a faithful lift.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use asm_runner::AsmRunnerRH;
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};
use sm_rom::RomInstance;
use zisk_common::{BusDevice, Instance, InstanceType, Stats, StatsScope};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

use crate::ports::{GlobalId, WitnessRegistry};
use crate::{
    state::ExecutionState, AirClassifier, ChunkDataCollector, StaticSMBundle, WitnessGenerator,
};

/// Type alias for the secondary instances map (owned).
type SecnInstanceMap<F> = HashMap<usize, Box<dyn Instance<F>>>;

/// Type alias for the secondary instances map (borrowed).
type SecnInstanceMapRef<'a, F> = HashMap<usize, &'a Box<dyn Instance<F>>>;

/// Context for witness computation operations.
///
/// `is_asm_emulator` is **not** carried here any more — the router
/// itself knows which backend it was built for.
///
/// `registry` is the ACL surface for `pctx` lookups
/// ([`crate::ports::Dctx`] / [`WitnessRegistry`]); `pctx` itself is
/// still kept because the downstream `WitnessGenerator` and
/// `ChunkDataCollector` call sites take `&ProofCtx<F>` directly
/// (cross-crate, library-coupled — out of scope for step 4.2).
pub struct WitnessContext<'a, F: PrimeField64> {
    /// Proof context (used by witness_generator / collector).
    pub pctx: &'a ProofCtx<F>,

    /// Setup context.
    pub sctx: &'a SetupCtx<F>,

    /// Execution state.
    pub state: &'a ExecutionState<F>,

    /// Buffer pool for trace data.
    pub buffer_pool: &'a dyn BufferPool<F>,

    /// Statistics scope.
    pub stats_scope: &'a StatsScope,

    /// ACL surface used by the router's own pctx-equivalent lookups
    /// (instance_info, set_witness_ready, is_my_process_instance, ...).
    pub registry: &'a dyn WitnessRegistry<F>,
}

impl<'a, F: PrimeField64> WitnessContext<'a, F> {
    /// Creates a new witness context.
    pub fn new(
        pctx: &'a ProofCtx<F>,
        sctx: &'a SetupCtx<F>,
        state: &'a ExecutionState<F>,
        buffer_pool: &'a dyn BufferPool<F>,
        stats_scope: &'a StatsScope,
        registry: &'a dyn WitnessRegistry<F>,
    ) -> Self {
        Self { pctx, sctx, state, buffer_pool, stats_scope, registry }
    }

    /// Gets instance info (airgroup_id, air_id) for a global ID.
    /// Routed via [`WitnessRegistry`] (which inherits from
    /// [`crate::ports::Dctx`]) — never touches `pctx` directly.
    pub fn get_instance_info(&self, global_id: usize) -> Result<(usize, usize)> {
        let info = self.registry.instance_info(GlobalId(global_id))?;
        Ok((info.airgroup_id, info.air_id))
    }
}

/// Phase-4 actor — dispatches witness computation per global id to
/// the right category-specific path (main / ROM-asm / ROM-native /
/// secondary / table).
///
/// The ASM-vs-native ROM choice is set once at construction via
/// [`Self::new_asm`] / [`Self::new_native`]; the dispatch then has no
/// per-call backend branching, only an air-id classification.
pub struct WitnessRouter<F: PrimeField64> {
    /// Chunk data collector for secondary instances.
    collector: ChunkDataCollector<F>,

    /// Witness computer for all instance types.
    witness_generator: WitnessGenerator,

    /// Reusable ROM trace buffer (single allocation across runs).
    trace_buffer_rom: Mutex<Vec<F>>,

    /// Backend the bundle was built for. ROM dispatch reads this.
    is_asm: bool,
}

impl<F: PrimeField64> WitnessRouter<F> {
    /// Construct a router bound to the **ASM** backend.
    /// ROM dispatch skips collection (RH service supplies the data
    /// out-of-band) and registers an empty collector.
    pub fn new_asm(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self::new(chunk_size, sm_bundle, true)
    }

    /// Construct a router bound to the **native (Rust)** backend.
    /// ROM dispatch runs normal collection.
    pub fn new_native(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self::new(chunk_size, sm_bundle, false)
    }

    /// Internal constructor used by the two public flavours.
    fn new(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>, is_asm: bool) -> Self {
        let collector = ChunkDataCollector::new(sm_bundle.clone());
        let witness_generator = WitnessGenerator::new(chunk_size);
        let trace_buffer_rom = Mutex::new(vec![F::ZERO; RomTrace::<F>::NUM_ROWS]);
        Self { collector, witness_generator, trace_buffer_rom, is_asm }
    }

    /// Returns `true` if the router was built for the ASM backend.
    #[inline]
    pub fn is_asm(&self) -> bool {
        self.is_asm
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

    /// Dispatches witness computation for a single global ID.
    ///
    /// Routes by air-id classification:
    ///   * **main** → `compute_main_witness`
    ///   * **secondary** (any other) → `compute_secondary_witness`
    ///     (which further branches on ROM + ASM vs native vs table).
    pub fn dispatch(&self, ctx: &WitnessContext<'_, F>, global_id: usize) -> Result<()> {
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
            )
        }
    }

    /// Computes witness for a main instance.
    fn compute_main_witness(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        global_id: usize,
        buffer_pool: &dyn BufferPool<F>,
        stats_scope: &StatsScope,
    ) -> Result<()> {
        let main_instances =
            state.instance_set.main_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
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
    /// Reads `self.is_asm` for the ROM-collection branch (ASM skips
    /// collection, native collects normally).
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
    ) -> Result<()> {
        let secn_instances =
            state.instance_set.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
        let secn_instance = secn_instances
            .get(&global_id)
            .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?;

        if secn_instance.instance_type() == InstanceType::Instance {
            let needs_collection = !state
                .collector_store
                .inner
                .read()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .contains_key(&global_id);

            if needs_collection {
                if AirClassifier::is_rom(air_id) && self.is_asm {
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
    fn register_empty_collector(
        &self,
        state: &ExecutionState<F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) -> Result<()> {
        let stats = Stats::new_no_collection(airgroup_id, air_id);

        state
            .collector_store
            .inner
            .write()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .insert(global_id, Vec::new());
        state.stats.insert_witness_stats(global_id, stats);

        Ok(())
    }

    /// Extracts collectors from state, returning an empty list for table instances.
    #[allow(clippy::type_complexity)]
    fn take_collectors_for_instance(
        state: &ExecutionState<F>,
        global_id: usize,
        instance_type: InstanceType,
    ) -> Result<Vec<(usize, Box<dyn BusDevice<u64>>)>> {
        match instance_type {
            InstanceType::Instance => {
                let mut guard =
                    state.collector_store.inner.write().map_err(|e| anyhow::anyhow!("{e}"))?;

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
    /// Reads `self.is_asm` for the ROM pre-calc branch.
    ///
    /// `pctx` is still required because [`ChunkDataCollector::collect`]
    /// takes `&ProofCtx<F>` directly (cross-crate, library-coupled — out
    /// of scope for step 4.2). All other `pctx`-equivalent lookups
    /// (`instance_info`, `set_witness_ready`) route through `registry`.
    pub fn pre_calculate(
        &self,
        pctx: &ProofCtx<F>,
        registry: &dyn WitnessRegistry<F>,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) -> Result<()> {
        let secn_instances_guard =
            state.instance_set.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut instances_to_collect = HashMap::new();

        for &global_id in global_ids {
            let info = registry.instance_info(GlobalId(global_id))?;

            if AirClassifier::is_main(info.air_id) {
                registry.set_witness_ready(GlobalId(global_id), false);
            } else if AirClassifier::is_rom(info.air_id) {
                self.handle_rom_pre_calculate(
                    registry,
                    state,
                    &secn_instances_guard,
                    &mut instances_to_collect,
                    global_id,
                    info.airgroup_id,
                    info.air_id,
                )?;
            } else {
                self.handle_secondary_pre_calculate(
                    registry,
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

    /// Handles ROM instance pre-calculation. Branches on `self.is_asm`.
    #[allow(clippy::too_many_arguments)]
    fn handle_rom_pre_calculate<'a>(
        &self,
        registry: &dyn WitnessRegistry<F>,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) -> Result<()> {
        let gid = GlobalId(global_id);
        if self.is_asm {
            registry.set_witness_ready(gid, false);
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
                registry.set_witness_ready(gid, true);
            } else {
                instances_to_collect.insert(global_id, secn_instance);
            }
        }

        Ok(())
    }

    /// Handles secondary instance pre-calculation.
    fn handle_secondary_pre_calculate<'a>(
        &self,
        registry: &dyn WitnessRegistry<F>,
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
                .collector_store
                .inner
                .read()
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .contains_key(&global_id)
        {
            instances_to_collect.insert(global_id, secn_instance);
        } else {
            registry.set_witness_ready(GlobalId(global_id), true);
        }

        Ok(())
    }
}
