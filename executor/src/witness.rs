//! [`WitnessPhase`] — phase-4 actor that classifies each global id
//! by air category and dispatches witness computation to the right
//! per-category path: main, secondary, ROM (shared body in
//! [`WitnessPhase::rom_dispatch`]), or table.
//!
//! The ROM-backend selection is read at runtime from the [`WitnessContext`]'s `is_asm_emulator` flag.

pub mod air_classifier;
pub mod collect_budget;
pub mod collector;
pub mod generator;
pub mod handlers;
pub mod host_memory;

pub use air_classifier::*;
pub use collector::*;
pub use generator::*;
pub use handlers::*;
pub use host_memory::{HostMemoryMargin, ParseHostMemoryMarginError};

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use proofman_common::{BufferPool, ProofCtx, SetupCtx, WitnessPriority};
use proofman_fields::PrimeField64;
use zisk_asm_runner::AsmRunnerRH;
use zisk_common::{
    CheckPoint, Instance, InstanceCtx, InstanceType, LateJoinHandle, LateValue, Plan, StatsScope,
};
use zisk_core::ZiskRom;
use zisk_pil::RomTrace;
use zisk_sm_main::MainInstance;
use zisk_sm_rom::RomInstance;

use crate::error::{ExecutorError, ExecutorResult, MutexExt, RwLockExt};
use crate::ports::{Dctx, GlobalId, ProofRegistry};
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
    /// (instance_info, announce_witness_ready, is_my_process_instance, ...).
    pub registry: &'a dyn Dctx,

    /// Runtime selector for the ROM backend
    pub is_asm_emulator: bool,
}

impl<'a, F: PrimeField64> WitnessContext<'a, F> {
    /// Creates a new witness context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pctx: &'a ProofCtx<F>,
        sctx: &'a SetupCtx<F>,
        state: &'a ExecutionState<F>,
        buffer_pool: &'a dyn BufferPool<F>,
        stats_scope: &'a StatsScope,
        registry: &'a dyn Dctx,
        is_asm_emulator: bool,
    ) -> Self {
        Self { pctx, sctx, state, buffer_pool, stats_scope, registry, is_asm_emulator }
    }

    /// Gets instance info (airgroup_id, air_id) for a global ID.
    /// Routed via [`crate::ports::Dctx`] — never touches `pctx` directly.
    pub fn get_instance_info(&self, global_id: usize) -> ExecutorResult<(usize, usize)> {
        let info = self.registry.instance_info(GlobalId(global_id))?;
        Ok((info.airgroup_id, info.air_id))
    }
}

/// How [`WitnessPhase::pre_calculate`] may collect the instances it is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectMode {
    /// Within the memory budget ([`collect_budget`]): in one pass when everything fits, else in
    /// waves, each waiting for proofman to compute what the earlier ones announced. Only for a
    /// caller that holds nothing those computations need; it falls back to one pass where proofman
    /// does not compute the announces while the call runs.
    Waves,
    /// What fits in the budget now, and nothing more: the rest stays uncollected and unannounced,
    /// and proofman passes it again to a later call. Never waits. For the first registration
    /// round, which collects inside the execution.
    FittingPrefix,
}

/// Phase-4 actor — classifies each global id and dispatches to one
/// of five [`handlers`] modules. Also owns the witness-time
/// materialization of instances (formerly `InstancePopulator`).
pub struct WitnessPhase<F: PrimeField64> {
    /// Constructed SM bundle. Held directly so the populator-style
    /// methods can dispatch `build_instance` / `configure_instances`
    /// without going through `collector`.
    sm_bundle: Arc<StaticSMBundle<F>>,

    /// Chunk data collector for secondary instances.
    collector: ChunkDataCollector<F>,

    /// Witness computer for all instance types.
    witness_generator: WitnessGenerator,

    /// Reusable ROM trace buffer (single allocation across runs).
    trace_buffer_rom: Mutex<Vec<F>>,

    /// The ROM witness may be recomputed in the same proof; the FROPS column is published once.
    frops_published: AtomicBool,

    /// Host memory left out of what the collect may use; the default margin when `None`.
    memory_margin: Option<HostMemoryMargin>,
}

impl<F: PrimeField64> WitnessPhase<F> {
    pub fn new(
        chunk_size: u64,
        sm_bundle: Arc<StaticSMBundle<F>>,
        memory_margin: Option<HostMemoryMargin>,
    ) -> Self {
        let collector = ChunkDataCollector::new(sm_bundle.clone());
        let witness_generator = WitnessGenerator::new(chunk_size);
        let trace_buffer_rom = Mutex::new(vec![F::ZERO; RomTrace::<F>::NUM_ROWS]);
        Self {
            sm_bundle,
            collector,
            witness_generator,
            trace_buffer_rom,
            frops_published: AtomicBool::new(false),
            memory_margin,
        }
    }

    /// Parks this execution's ASM ROM-histogram runner on the ROM state machine.
    ///
    /// Straight to the bundle: the collector has no part in a handle that is read at
    /// witness time. See [`StaticSMBundle::park_rh_handle`].
    pub fn park_rh_handle(&self, handle: LateJoinHandle<AsmRunnerRH>) -> ExecutorResult<()> {
        self.sm_bundle.park_rh_handle(handle)
    }

    /// See [`StaticSMBundle::arm_frops_cross_check`].
    pub fn arm_frops_cross_check(&self) -> ExecutorResult<()> {
        self.sm_bundle.arm_frops_cross_check()
    }

    /// The cell the ROM-histogram runner is parked in; see
    /// [`StaticSMBundle::rom_histogram`].
    pub fn rom_histogram(&self) -> Option<Arc<LateValue<AsmRunnerRH>>> {
        self.sm_bundle.rom_histogram()
    }

    /// Selects where the FROPS multiplicity column comes from; see
    /// `StaticSMBundle::set_frops_multiplicity_from_asm`.
    pub fn set_frops_multiplicity_from_asm(&self, from_asm: bool) {
        self.collector.set_frops_multiplicity_from_asm(from_asm)
    }

    pub fn set_rom(&self, zisk_rom: Arc<ZiskRom>) -> ExecutorResult<()> {
        self.collector.set_rom(zisk_rom.clone())
    }

    pub fn set_packed(&self, packed: bool) {
        self.witness_generator.set_packed(packed);
    }

    pub fn is_packed(&self) -> bool {
        self.witness_generator.is_packed()
    }

    pub fn reset(&self) -> ExecutorResult<()> {
        *self.trace_buffer_rom.lock_or_poison("trace_buffer_rom")? =
            vec![F::ZERO; RomTrace::<F>::NUM_ROWS];
        self.frops_published.store(false, Ordering::Relaxed);

        Ok(())
    }

    /// Materialise main instances into `state` and announce the ones this rank owns.
    pub fn populate_main_instances(
        &self,
        registry: &dyn ProofRegistry,
        state: &ExecutionState<F>,
        assignments: Vec<(usize, Plan)>,
    ) -> ExecutorResult<()> {
        let mut main_instances =
            state.instance_set.main_instances.write_or_poison("main_instances")?;
        for (global_id, plan) in assignments {
            main_instances.entry(global_id).or_insert_with(|| {
                std::sync::Arc::new(MainInstance::new(InstanceCtx::new(global_id, plan)))
            });

            let gid = GlobalId(global_id);
            if registry.is_my_process_instance(gid)? {
                registry.announce_witness_ready(gid);
            }
        }
        Ok(())
    }

    /// Configure secondary SMs on `pctx`. Called before flatten + GID assignment.
    pub fn configure_sm_instances(
        &self,
        pctx: &ProofCtx<F>,
        plannings: &BTreeMap<usize, Vec<Plan>>,
    ) {
        self.sm_bundle.configure_instances(pctx, plannings);
    }

    /// Materialise secondary instances into `state`. Plans must carry stamped `global_id`s.
    pub fn populate_secn_instances(
        &self,
        state: &ExecutionState<F>,
        plans: Vec<Plan>,
    ) -> ExecutorResult<()> {
        let mut secn_instances =
            state.instance_set.secn_instances.write_or_poison("secn_instances")?;
        for plan in plans {
            let global_id =
                plan.global_id.ok_or(ExecutorError::SecnPlanMissing { phase: "populate" })?;
            if let Entry::Vacant(e) = secn_instances.entry(global_id) {
                let instance = self.sm_bundle.build_instance(InstanceCtx::new(global_id, plan))?;
                e.insert(instance);
            }
        }
        Ok(())
    }

    /// Reset each secondary instance and register its chunks on the registry.
    pub fn configure_checkpoints(
        &self,
        registry: &dyn ProofRegistry,
        state: &ExecutionState<F>,
        global_ids: &[usize],
    ) -> ExecutorResult<()> {
        let secn_instances = state.instance_set.secn_instances.read_or_poison("secn_instances")?;

        for &global_id in global_ids {
            let instance = secn_instances
                .get(&global_id)
                .ok_or(ExecutorError::InstanceNotFound { global_id })?;

            instance.reset();

            if instance.instance_type() == InstanceType::Instance {
                let chunks: Vec<usize> = match instance.check_point() {
                    CheckPoint::None => vec![],
                    CheckPoint::Single(chunk_id) => vec![chunk_id.as_usize()],
                    CheckPoint::Multiple(chunk_ids) => {
                        chunk_ids.iter().map(|id| id.as_usize()).collect()
                    }
                };

                let gid = GlobalId(global_id);
                let info = registry.instance_info(gid)?;
                let is_memory_related = AirClassifier::is_memory_related(info.air_id);
                registry.set_chunks(gid, &chunks, is_memory_related);
            }
        }
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
                        !AirClassifier::mem_collected_on_device(air_id),
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

        // The ROM backend is picked when the instance is built; this is where an
        // ASM execution finds out it got the wrong one. See `require_asm_rom_mode`.
        if ctx.is_asm_emulator {
            require_asm_rom_mode(instance, global_id, air_id)?;
        }

        if needs_collection {
            if ctx.is_asm_emulator {
                // ASM ROM: the RH service supplies the data — pin an
                // empty slot so the rest of the pipeline observes the
                // "no-collection" state.
                ctx.state.register_empty_collector(global_id, airgroup_id, air_id)?;
            } else {
                self.collector.collect_single(ctx.pctx, ctx.state, global_id, instance)?;
            }
        }

        let collectors =
            ctx.state.take_collectors_for_instance(global_id, instance.instance_type())?;
        let trace_buffer =
            std::mem::take(&mut *self.trace_buffer_rom.lock_or_poison("trace_buffer_rom")?);

        self.witness_generator
            .compute_secn_witness(
                ctx.pctx,
                ctx.sctx,
                ctx.state,
                global_id,
                instance,
                collectors,
                trace_buffer,
                stats_scope_id,
            )
            .map_err(|(e, _)| e)?;

        // After the trace is handed over, so the ROM proof does not wait on the copy.
        if !self.frops_published.swap(true, Ordering::Relaxed) {
            self.sm_bundle.publish_frops_from_asm()?;
        }
        Ok(())
    }

    /// Pre-calculates witnesses by determining which instances need collection.
    ///
    /// `pctx` is still required because [`ChunkDataCollector::collect`]
    /// takes `&ProofCtx<F>` directly (cross-crate, library-coupled), and
    /// because the [`CollectMode::Waves`] budget reads the proof's runtime
    /// state from it. All other `pctx`-equivalent lookups (`instance_info`,
    /// `announce_witness_ready`) route through `registry`.
    pub fn pre_calculate(
        &self,
        pctx: &ProofCtx<F>,
        registry: &dyn Dctx,
        state: &ExecutionState<F>,
        global_ids: &[usize],
        is_asm_emulator: bool,
        mode: CollectMode,
    ) -> ExecutorResult<()> {
        let secn_instances_guard =
            state.instance_set.secn_instances.read_or_poison("secn_instances")?;

        let mut instances_to_collect = HashMap::new();

        for &global_id in global_ids {
            let info = registry.instance_info(GlobalId(global_id))?;

            // Filled on the device: nothing to collect, ready as is.
            if AirClassifier::mem_collected_on_device(info.air_id) {
                registry.announce_witness_ready(GlobalId(global_id));
                continue;
            }
            if AirClassifier::is_main(info.air_id) {
                registry.announce_witness_ready(GlobalId(global_id));
            } else if AirClassifier::is_rom(info.airgroup_id, info.air_id) {
                if is_asm_emulator {
                    // ASM ROM: the RH service handles collection out-of-band.
                    registry.announce_witness_ready(GlobalId(global_id));
                } else {
                    handlers::rom_rust::pre_calculate(
                        &secn_instances_guard,
                        &mut instances_to_collect,
                        global_id,
                        info.air_id,
                    )?;
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

        if instances_to_collect.is_empty() {
            return Ok(());
        }
        let plan = match mode {
            // Waves wait for proofman to compute what earlier waves announced: only where it
            // computes the announces on its own threads while this call runs (`witness_tx` set: its
            // admission thread is live). Anywhere else nobody would compute them in time. A single
            // instance makes a single wave, whatever its size.
            CollectMode::Waves
                if instances_to_collect.len() > 1
                    && pctx.witness_tx.read_or_poison("witness_tx")?.is_some() =>
            {
                self.plan_waves(pctx, registry, &instances_to_collect, 0, "collect")?
            }
            CollectMode::Waves => None,
            // Even a single instance: one that does not fit is left for later.
            CollectMode::FittingPrefix => self.plan_waves(
                pctx,
                registry,
                &instances_to_collect,
                collect_budget::FIRST_ROUND_RESERVE,
                "first-round collect",
            )?,
        };
        let Some(plan) = plan else {
            return self.collector.collect(pctx, state, instances_to_collect);
        };
        if mode == CollectMode::FittingPrefix {
            return self.collect_fitting_prefix(pctx, state, &instances_to_collect, &plan);
        }
        // The waves wait for proofman's witness threads, which read the instances too: hold the
        // instances only while each wave is collected, never across a wait.
        drop(instances_to_collect);
        drop(secn_instances_guard);
        self.collect_waves(pctx, state, plan)
    }

    /// `instances` in the order to collect them within the budget, or `None` to collect them in one
    /// pass: when they fit, or when there is no budget to keep. The automatic budget also leaves
    /// `reserve` for work that runs alongside; `label` names the collect in the log.
    fn plan_waves(
        &self,
        pctx: &ProofCtx<F>,
        registry: &dyn Dctx,
        instances: &HashMap<usize, &dyn Instance<F>>,
        reserve: u64,
        label: &str,
    ) -> ExecutorResult<Option<collect_budget::WavePlan>> {
        let node_processes = pctx.mpi_ctx.node_n_processes.max(1) as usize;
        let host_share = || host_memory::host_memory_share(node_processes, self.memory_margin);
        let Some(mut budget) = collect_budget::collect_budget(reserve, host_share) else {
            return Ok(None);
        };
        let total: u64 = instances.values().map(|i| i.collector_capacity_bytes()).sum();
        if total > budget && zisk_common::allocator::release_free_memory() {
            // The allocator keeps the pages of the buffers freed so far resident, to serve its next
            // allocations, and the OS counts them as used: hand them back and measure again.
            let before = budget;
            budget = collect_budget::collect_budget(reserve, host_share).unwrap_or(before);
            tracing::info!(
                "{label}: released the allocator's free pages: budget {:.1} -> {:.1} GiB",
                collect_budget::gib(before),
                collect_budget::gib(budget)
            );
        }
        if total <= budget {
            tracing::info!(
                "{label}: {} instances reserve {:.1} GiB of a {:.1} GiB budget: one pass",
                instances.len(),
                collect_budget::gib(total),
                collect_budget::gib(budget)
            );
            return Ok(None);
        }

        // `Last` instances go last: proofman computes them only after every other instance, so a
        // wave waiting on one would wait for itself. The memory instances, which touch chunks from
        // the whole block, follow the rest, which go in block order to replay contiguous chunks.
        let mut order = Vec::with_capacity(instances.len());
        for (&global_id, instance) in instances {
            let air_id = registry.instance_info(GlobalId(global_id))?.air_id;
            let last = pctx.dctx_instance_priority(global_id) == WitnessPriority::Last;
            let memory = AirClassifier::is_memory_related(air_id);
            let first_chunk = match instance.check_point() {
                CheckPoint::Single(chunk) => chunk.as_usize(),
                CheckPoint::Multiple(chunks) => {
                    chunks.iter().map(|c| c.as_usize()).min().unwrap_or(0)
                }
                CheckPoint::None => 0,
            };
            let pending =
                collect_budget::Pending { global_id, bytes: instance.collector_capacity_bytes() };
            order.push(((last, memory, first_chunk, global_id), pending));
        }
        order.sort_unstable_by_key(|(key, _)| *key);
        let pending = order.into_iter().map(|(_, pending)| pending).collect();
        Ok(Some(collect_budget::WavePlan { budget, total, pending }))
    }

    /// Collects the front of `plan` that fits in its budget, and leaves the rest uncollected and
    /// unannounced for proofman to pass again to a later call. Never waits.
    fn collect_fitting_prefix(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        instances: &HashMap<usize, &dyn Instance<F>>,
        plan: &collect_budget::WavePlan,
    ) -> ExecutorResult<()> {
        let take = collect_budget::fitting_prefix(&plan.pending, plan.budget);
        let (now, later) = plan.pending.split_at(take);
        tracing::info!(
            "first-round collect: {} instances reserve {:.1} GiB of a {:.1} GiB budget: {} now, {} ({:.1} GiB) left for later",
            plan.pending.len(),
            collect_budget::gib(plan.total),
            collect_budget::gib(plan.budget),
            now.len(),
            later.len(),
            collect_budget::gib(later.iter().map(|p| p.bytes).sum())
        );
        if now.is_empty() {
            return Ok(());
        }
        let prefix = now.iter().map(|p| (p.global_id, instances[&p.global_id])).collect();
        self.collector.collect(pctx, state, prefix)
    }

    /// Collects `plan` wave by wave, each one once the instances the earlier ones announced have
    /// freed enough of the budget.
    fn collect_waves(
        &self,
        pctx: &ProofCtx<F>,
        state: &ExecutionState<F>,
        plan: collect_budget::WavePlan,
    ) -> ExecutorResult<()> {
        let started = std::time::Instant::now();
        let mut waited = std::time::Duration::ZERO;
        let mut held = collect_budget::Held::default();
        let mut waves = 0;
        let mut rest = &plan.pending[..];
        while !rest.is_empty() {
            if !held.is_empty() {
                let wait_started = std::time::Instant::now();
                collect_budget::wait_for_room(
                    &mut held,
                    rest[0].bytes,
                    plan.budget,
                    |global_id| pctx.dctx_witness_state(global_id),
                    || pctx.is_cancelled(),
                )?;
                waited += wait_started.elapsed();
            }
            let take = collect_budget::next_wave(rest, plan.budget.saturating_sub(held.bytes()));
            let (wave, tail) = rest.split_at(take);
            {
                let secn_instances =
                    state.instance_set.secn_instances.read_or_poison("secn_instances")?;
                let wave_instances = wave
                    .iter()
                    .map(|p| {
                        secn_instances
                            .get(&p.global_id)
                            .map(|instance| (p.global_id, &**instance))
                            .ok_or(ExecutorError::InstanceNotFound { global_id: p.global_id })
                    })
                    .collect::<ExecutorResult<HashMap<_, _>>>()?;
                self.collector.collect(pctx, state, wave_instances)?;
            }
            held.add(wave);
            waves += 1;
            rest = tail;
        }
        tracing::info!(
            "collect: {} instances reserve {:.1} GiB of a {:.1} GiB budget: {} waves in {:.1} s, {:.1} s waiting",
            plan.pending.len(),
            collect_budget::gib(plan.total),
            collect_budget::gib(plan.budget),
            waves,
            started.elapsed().as_secs_f64(),
            waited.as_secs_f64()
        );
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
            registry.announce_witness_ready(GlobalId(global_id));
        }

        Ok(())
    }
}

/// Fails an ASM execution whose ROM instance was built for the Rust backend.
///
/// On the ASM path the ROM witness comes from the assembly histogram, and
/// [`WitnessPhase::rom_dispatch`] registers an empty collector because nothing is
/// meant to fill one. A Rust-backend instance reaching that path therefore
/// aggregates no collectors at all and computes an **all-zero ROM trace**, which
/// then gets proved without complaint. The backend is chosen when the instance is
/// built, from whether the histogram is available by then, so this can only mean
/// the histogram was missing at build time — a lifecycle bug, not a valid mode.
///
/// A free function so it can be exercised on a bare [`RomInstance`], with no proof
/// or witness context to construct.
fn require_asm_rom_mode<F: PrimeField64>(
    instance: &dyn Instance<F>,
    global_id: usize,
    air_id: usize,
) -> ExecutorResult<()> {
    let rom_instance =
        crate::sm::downcast::<F, RomInstance>(instance, air_id, global_id, "RomInstance")?;

    // `skip_collector` is the instance's own name for "my witness comes from the
    // assembly histogram" — the ASM backend, and the only correct one here.
    if rom_instance.skip_collector() {
        Ok(())
    } else {
        Err(ExecutorError::RomBackendDowngrade { global_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::witness::handlers::rom_rust::tests::{make_rom_instance, AIR_ID, GID};
    use proofman_fields::Goldilocks;
    use zisk_asm_runner::{AsmRHData, AsmRunnerRH};

    type F = Goldilocks;

    /// An ASM-backend ROM instance, i.e. one built while a histogram runner was parked.
    fn asm_rom_instance() -> Box<dyn Instance<F>> {
        make_rom_instance(Some(AsmRunnerRH::new(AsmRHData::new(0, Vec::new(), Vec::new()))))
    }

    #[test]
    fn asm_backend_rom_instance_passes() {
        let instance = asm_rom_instance();
        require_asm_rom_mode::<F>(&*instance, GID, AIR_ID)
            .expect("the ASM backend is what an ASM execution must find");
    }

    #[test]
    fn rust_backend_rom_instance_is_rejected() {
        let instance = make_rom_instance(None);
        let err = require_asm_rom_mode::<F>(&*instance, GID, AIR_ID)
            .expect_err("a Rust-backend instance here would prove an all-zero ROM trace");
        assert!(
            matches!(err, ExecutorError::RomBackendDowngrade { global_id: GID }),
            "got {err:?}"
        );
    }
}
