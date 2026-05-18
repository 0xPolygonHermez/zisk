//! [`WitnessRouter`] — phase-4 actor that dispatches witness
//! computation per global id to the right per-category handler in
//! [`crate::witness_handlers`].
//!
//! Step 4.3: the per-category compute logic that used to live inline
//! in `compute_main_witness` / `compute_secondary_witness` is now
//! five focused handler modules, each in its own file. The router
//! classifies the air id (and, for secondary, the `InstanceType` and
//! ROM-ness) and forwards to the matching handler.
//!
//! Construction-time `new_asm` / `new_native` still pick the ROM
//! handler once per executor; the dispatch path has no per-call
//! backend branching beyond the ROM cases.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use asm_runner::AsmRunnerRH;
use fields::PrimeField64;
use proofman_common::{BufferPool, ProofCtx, SetupCtx};
use sm_rom::RomInstance;
use zisk_common::{InstanceType, StatsScope};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;

use crate::ports::{GlobalId, WitnessRegistry};
use crate::witness_handlers::{
    common::register_empty_collector, MainWitnessHandler, RomAsmWitnessHandler,
    RomNativeWitnessHandler, RomWitnessHandler, SecnInstanceMap, SecnInstanceMapRef,
    SecondaryWitnessHandler, TableWitnessHandler,
};
use crate::{
    state::ExecutionState, AirClassifier, ChunkDataCollector, StaticSMBundle, WitnessGenerator,
};

/// Context for witness computation operations.
///
/// `registry` is the ACL surface for `pctx` lookups
/// ([`crate::ports::Dctx`] / [`WitnessRegistry`]); `pctx` itself is
/// still kept because the downstream `WitnessGenerator` and
/// `ChunkDataCollector` call sites take `&ProofCtx<F>` directly
/// (cross-crate, library-coupled).
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

/// Phase-4 actor — classifies each global id and dispatches to one
/// of five [`crate::witness_handlers`] modules.
pub struct WitnessRouter<F: PrimeField64> {
    /// Chunk data collector for secondary instances. Shared by the
    /// secondary + rom_native handlers (passed by reference).
    collector: ChunkDataCollector<F>,

    /// Witness computer for all instance types.
    witness_generator: WitnessGenerator,

    /// Reusable ROM trace buffer (single allocation across runs),
    /// shared by both ROM handlers.
    trace_buffer_rom: Mutex<Vec<F>>,

    /// Backend the bundle was built for. Picks ROM-asm vs ROM-native
    /// handler at dispatch.
    is_asm: bool,
}

impl<F: PrimeField64> WitnessRouter<F> {
    /// Construct a router bound to the **ASM** backend.
    /// ROM dispatch routes to [`RomAsmWitnessHandler`].
    pub fn new_asm(chunk_size: u64, sm_bundle: Arc<StaticSMBundle<F>>) -> Self {
        Self::new(chunk_size, sm_bundle, true)
    }

    /// Construct a router bound to the **native (Rust)** backend.
    /// ROM dispatch routes to [`RomNativeWitnessHandler`].
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
    /// Classification ladder:
    ///   1. `is_main(air_id)` → [`MainWitnessHandler`]
    ///   2. Otherwise look up the secondary instance:
    ///      * `Table` → [`TableWitnessHandler`]
    ///      * `Instance` + `is_rom(air_id)` →
    ///        [`RomAsmWitnessHandler`] (if `self.is_asm`) or
    ///        [`RomNativeWitnessHandler`] (otherwise)
    ///      * `Instance` (non-ROM) → [`SecondaryWitnessHandler`]
    pub fn dispatch(&self, ctx: &WitnessContext<'_, F>, global_id: usize) -> Result<()> {
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
            let secn =
                ctx.state.instance_set.secn_instances.read().map_err(|e| anyhow::anyhow!("{e}"))?;
            secn.get(&global_id)
                .ok_or_else(|| anyhow::anyhow!("Instance not found: global_id={global_id}"))?
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
                if AirClassifier::is_rom(air_id) {
                    // Both handlers implement `RomWitnessHandler<F>` with a
                    // unified signature; C.2 will collapse this branch into
                    // a single boxed-handler call held on the router.
                    let handler: &dyn RomWitnessHandler<F> = if self.is_asm {
                        &RomAsmWitnessHandler
                    } else {
                        &RomNativeWitnessHandler
                    };
                    handler.dispatch(
                        &self.witness_generator,
                        &self.collector,
                        &self.trace_buffer_rom,
                        ctx.state,
                        ctx.pctx,
                        ctx.sctx,
                        global_id,
                        airgroup_id,
                        air_id,
                        stats_scope_id,
                    )
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

    /// Pre-calculates witnesses by determining which instances need collection.
    ///
    /// Reads `self.is_asm` for the ROM pre-calc branch.
    ///
    /// `pctx` is still required because [`ChunkDataCollector::collect`]
    /// takes `&ProofCtx<F>` directly (cross-crate, library-coupled).
    /// All other `pctx`-equivalent lookups (`instance_info`,
    /// `set_witness_ready`) route through `registry`.
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
                register_empty_collector(state, global_id, airgroup_id, air_id)?;
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
