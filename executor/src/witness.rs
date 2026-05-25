//! [`WitnessPhase`] — phase-4 actor that classifies each global id
//! by air category and dispatches witness computation to the right
//! per-category path: main, secondary, ROM (shared body in
//! [`WitnessPhase::rom_dispatch`]), or table.
//!
//! `new_asm` / `new_rust` pick the ROM backend once per executor;
//! only the in-collection step of `rom_dispatch` branches on it at
//! runtime.

pub mod air_classifier;
pub mod collector;
pub mod generator;
pub mod handlers;

pub use air_classifier::*;
pub use collector::*;
pub use generator::*;
pub use handlers::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use asm_runner::AsmRunnerRH;
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};
use zisk_common::{InstanceType, StatsScope};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

use crate::error::{ExecutorError, ExecutorResult, MutexExt, RwLockExt};
use crate::ports::{Dctx, GlobalId};
use crate::sm::StaticSMBundle;
use crate::state::ExecutionState;

/// Context for witness computation operations.
///
/// `registry` is the ACL surface for `pctx` lookups
/// ([`crate::ports::Dctx`]); `pctx` itself is still kept because the
/// downstream `WitnessGenerator` and `ChunkDataCollector` call sites
/// take `&ProofCtx<F>` directly (cross-crate, library-coupled).
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
    pub registry: &'a dyn Dctx,
}

impl<'a, F: PrimeField64> WitnessContext<'a, F> {
    /// Creates a new witness context.
    pub fn new(
        pctx: &'a ProofCtx<F>,
        sctx: &'a SetupCtx<F>,
        state: &'a ExecutionState<F>,
        buffer_pool: &'a dyn BufferPool<F>,
        stats_scope: &'a StatsScope,
        registry: &'a dyn Dctx,
    ) -> Self {
        Self { pctx, sctx, state, buffer_pool, stats_scope, registry }
    }

    /// Gets instance info (airgroup_id, air_id) for a global ID.
    /// Routed via [`crate::ports::Dctx`] — never touches `pctx` directly.
    pub fn get_instance_info(&self, global_id: usize) -> ExecutorResult<(usize, usize)> {
        let info = self.registry.instance_info(GlobalId(global_id))?;
        Ok((info.airgroup_id, info.air_id))
    }
}

/// Phase-4 actor — classifies each global id and dispatches to one
/// of five [`handlers`] modules.
pub struct WitnessPhase<F: PrimeField64> {
    /// Chunk data collector for secondary instances. Shared by the
    /// secondary + Rust ROM handlers (passed by reference).
    collector: ChunkDataCollector<F>,

    /// Witness computer for all instance types.
    witness_generator: WitnessGenerator,

    /// Reusable ROM trace buffer (single allocation across runs),
    /// shared by both ROM handlers.
    trace_buffer_rom: Mutex<Vec<F>>,

    /// Backend tag chosen once at construction by `new_asm` /
    /// `new_rust`. Selects the one backend-specific branch in
    /// [`Self::rom_dispatch`] and which `pre_calculate` free function
    /// to call.
    rom_backend: RomBackend,
}

impl<F: PrimeField64> WitnessPhase<F> {
    /// Construct a router bound to the **ASM** backend.
    pub fn new_asm(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self::new(chunk_size, sm_bundle, RomBackend::Asm)
    }

    /// Construct a router bound to the **Rust** backend.
    pub fn new_rust(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self::new(chunk_size, sm_bundle, RomBackend::Rust)
    }

    /// Internal constructor used by the two public flavours.
    fn new(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>, rom_backend: RomBackend) -> Self {
        let collector = ChunkDataCollector::new(sm_bundle.clone());
        let witness_generator = WitnessGenerator::new(chunk_size);
        let trace_buffer_rom = Mutex::new(vec![F::ZERO; RomTrace::<F>::NUM_ROWS]);
        Self { collector, witness_generator, trace_buffer_rom, rom_backend }
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) -> ExecutorResult<()> {
        self.collector.set_rh_data(rh_data)
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        self.collector.set_rom(zisk_rom.clone())
    }

    pub fn set_packed(&self, packed: bool) {
        self.witness_generator.set_packed(packed);
    }

    pub fn reset(&self) -> ExecutorResult<()> {
        *self.trace_buffer_rom.lock_or_poison("trace_buffer_rom")? =
            vec![F::ZERO; RomTrace::<F>::NUM_ROWS];

        Ok(())
    }

    /// Dispatches witness computation for a single global ID.
    ///
    /// Classification ladder:
    ///   1. `is_main(air_id)` → [`MainWitnessHandler`]
    ///   2. Otherwise look up the secondary instance:
    ///      * `Table` → [`TableWitnessHandler`]
    ///      * `Instance` + `is_rom(air_id)` → `self.rom_handler`
    ///        (the strategy baked at construction)
    ///      * `Instance` (non-ROM) → [`SecondaryWitnessHandler`]
    pub fn dispatch(&self, ctx: &WitnessContext<'_, F>, global_id: usize) -> ExecutorResult<()> {
        let (airgroup_id, air_id) = ctx.get_instance_info(global_id)?;
        let stats_scope_id = ctx.stats_scope.id();

        if AirClassifier::is_main(air_id) {
            return MainWitnessHandler::dispatch(
                &self.witness_generator,
                ctx.state,
                ctx.pctx,
                global_id,
                ctx.buffer_pool,
                stats_scope_id,
            );
        }

        // Secondary path: look up the instance's `InstanceType` (so we
        // can route Table separately from Instance) without holding the
        // read guard across the handler call.
        let instance_type = {
            let secn = ctx.state.instance_set.secn_instances.read_or_poison("secn_instances")?;
            secn.get(&global_id)
                .ok_or(ExecutorError::InstanceNotFound { global_id })?
                .instance_type()
        };

        match instance_type {
            InstanceType::Table => TableWitnessHandler::dispatch(
                &self.witness_generator,
                ctx.state,
                ctx.pctx,
                ctx.sctx,
                global_id,
                ctx.buffer_pool,
                stats_scope_id,
            ),
            InstanceType::Instance => {
                if AirClassifier::is_rom(airgroup_id, air_id) {
                    self.rom_dispatch(ctx, global_id, airgroup_id, air_id, stats_scope_id)
                } else {
                    SecondaryWitnessHandler::dispatch(
                        &self.witness_generator,
                        &self.collector,
                        ctx.state,
                        ctx.pctx,
                        ctx.sctx,
                        global_id,
                        ctx.buffer_pool,
                        stats_scope_id,
                    )
                }
            }
        }
    }

    /// ROM witness compute — one algorithm, one backend-specific
    /// branch. The lookup / take-collectors / `compute_secn_witness`
    /// scaffolding is shared between ASM and Rust; only the action
    /// taken when collection is needed differs.
    fn rom_dispatch(
        &self,
        ctx: &WitnessContext<'_, F>,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
        stats_scope_id: u64,
    ) -> ExecutorResult<()> {
        let secn_instances =
            ctx.state.instance_set.secn_instances.read_or_poison("secn_instances")?;
        let secn_instance =
            secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;

        let needs_collection = !ctx
            .state
            .collector_store
            .inner
            .read_or_poison("collector_store")?
            .contains_key(&global_id);

        let instance = &**secn_instance;
        if needs_collection {
            match self.rom_backend {
                // ASM ROM: the RH service supplies the data — pin an
                // empty slot so the rest of the pipeline observes the
                // "no-collection" state.
                RomBackend::Asm => {
                    ctx.state.register_empty_collector(global_id, airgroup_id, air_id)?;
                }
                RomBackend::Rust => {
                    self.collector.collect_single(ctx.pctx, ctx.state, global_id, instance)?;
                }
            }
        }

        let collectors =
            ctx.state.take_collectors_for_instance(global_id, instance.instance_type())?;
        let trace_buffer =
            std::mem::take(&mut *self.trace_buffer_rom.lock_or_poison("trace_buffer_rom")?);

        self.witness_generator.compute_secn_witness(
            ctx.pctx,
            ctx.sctx,
            ctx.state,
            global_id,
            instance,
            collectors,
            trace_buffer,
            stats_scope_id,
        )
    }

    /// Pre-calculates witnesses by determining which instances need collection.
    ///
    /// ROM-id pre-calc dispatches on `rom_backend` to the matching free
    /// function in [`handlers::rom_asm`] / [`handlers::rom_rust`]. The
    /// two paths share nothing structural, so they're not unified.
    ///
    /// `pctx` is still required because [`ChunkDataCollector::collect`]
    /// takes `&ProofCtx<F>` directly (cross-crate, library-coupled).
    /// All other `pctx`-equivalent lookups (`instance_info`,
    /// `set_witness_ready`) route through `registry`.
    pub fn pre_calculate(
        &self,
        pctx: &ProofCtx<F>,
        registry: &dyn Dctx,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) -> ExecutorResult<()> {
        let secn_instances_guard =
            state.instance_set.secn_instances.read_or_poison("secn_instances")?;

        let mut instances_to_collect = HashMap::new();

        for &global_id in global_ids {
            let info = registry.instance_info(GlobalId(global_id))?;

            if AirClassifier::is_main(info.air_id) {
                registry.set_witness_ready(GlobalId(global_id), false);
            } else if AirClassifier::is_rom(info.airgroup_id, info.air_id) {
                match self.rom_backend {
                    // ASM ROM: the RH service handles collection
                    // out-of-band; just flag the gid not-ready.
                    RomBackend::Asm => {
                        registry.set_witness_ready(GlobalId(global_id), false);
                    }
                    RomBackend::Rust => handlers::rom_rust::pre_calculate(
                        registry,
                        state,
                        &secn_instances_guard,
                        &mut instances_to_collect,
                        global_id,
                        info.airgroup_id,
                        info.air_id,
                    )?,
                }
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
            self.collector.collect(pctx, state, instances_to_collect)?;
        }

        Ok(())
    }

    /// Handles secondary instance pre-calculation.
    fn handle_secondary_pre_calculate<'a>(
        &self,
        registry: &dyn Dctx,
        state: &ExecutionState<F>,
        secn_instances: &'a SecnInstanceMap<F>,
        instances_to_collect: &mut SecnInstanceMapRef<'a, F>,
        global_id: usize,
    ) -> ExecutorResult<()> {
        let secn_instance =
            secn_instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;

        if secn_instance.instance_type() == InstanceType::Instance
            && !state
                .collector_store
                .inner
                .read_or_poison("collector_store")?
                .contains_key(&global_id)
        {
            instances_to_collect.insert(global_id, &**secn_instance);
        } else {
            registry.set_witness_ready(GlobalId(global_id), true);
        }

        Ok(())
    }
}
