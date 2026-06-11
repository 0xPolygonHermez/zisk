use std::{
    any::Any,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use fields::Goldilocks;
use proofman_common::{create_pool, BufferPool, ProofCtx, ProofmanError, ProofmanResult, SetupCtx};
use witness::{WitnessComponent, WitnessManager};

use crate::{
    unit_test_hooks::UnitTestHookBag,
    unit_test_targets::{
        build_manager_registry, lookup_by_air_id, lookup_by_name, lookup_override_by_air_id,
    },
    unit_test_trace_override::TraceOverrideBag,
    Precompiles, StaticSMBundle,
};

/// Alternative `WitnessComponent` used by the unit-test backend.
///
/// The struct is concrete to `Goldilocks` because the registry of per-SM
/// trait impls is concrete to `Goldilocks` and we never instantiate the
/// unit-test path with any other field. Keeping it concrete avoids a layer
/// of generic bounds that would otherwise have to be threaded through every
/// call site.
pub struct ZiskExecutorTest {
    #[allow(dead_code)]
    sm_bundle: Arc<StaticSMBundle<Goldilocks>>,
    /// Per-AIR-id erased Manager arcs, built once from the bundle.
    /// Indexed by AIR id; the dispatcher passes the right one to each
    /// trait method based on the planned instance.
    managers: HashMap<usize, Arc<dyn Any + Send + Sync>>,
    /// Per-instance erased input chunks, keyed by global_id.
    inputs: RwLock<HashMap<usize, Box<dyn Any + Send + Sync>>>,
    /// Per-instance AIR id (so `calculate_witness` can look up the right
    /// SM in the registry without touching pctx again).
    air_ids: RwLock<HashMap<usize, usize>>,
    /// Type-erased input data set by the `verify_input()` builder before each
    /// run (per-SM batches of boxed `S::Input`). The only input source for the
    /// unit-test executor.
    input_data_value: RwLock<Option<HashMap<String, Vec<Box<dyn Any + Send + Sync>>>>>,
    /// Per-AIR-id post-hoc trace-row hooks. Empty by default; set via
    /// [`ZiskExecutorTest::set_hooks`]. Cleared at the start of every
    /// `execute()` so leftover hooks from a previous run don't apply.
    hooks: RwLock<UnitTestHookBag>,
    /// Per-AIR-id raw trace-authoring overrides. Empty by default; set via
    /// [`ZiskExecutorTest::set_trace_overrides`]. When an override is present
    /// for an AIR id, the executor bypasses that SM's `compute_witness` and
    /// builds the `AirInstance` from the override closure instead.
    trace_overrides: RwLock<TraceOverrideBag>,
    packed: AtomicBool,
}

impl ZiskExecutorTest {
    /// Build a unit-test executor from a constructed SM bundle, eagerly
    /// building the AIR-id → inner-SM manager registry from it.
    pub fn new(sm_bundle: StaticSMBundle<Goldilocks>) -> Self {
        let managers = build_manager_registry(&sm_bundle);
        Self {
            sm_bundle: Arc::new(sm_bundle),
            managers,
            inputs: RwLock::new(HashMap::new()),
            air_ids: RwLock::new(HashMap::new()),
            input_data_value: RwLock::new(None),
            hooks: RwLock::new(UnitTestHookBag::new()),
            trace_overrides: RwLock::new(TraceOverrideBag::new()),
            packed: AtomicBool::new(false),
        }
    }

    fn manager_for(&self, air_id: usize) -> ProofmanResult<&Arc<dyn Any + Send + Sync>> {
        self.managers.get(&air_id).ok_or_else(|| {
            ProofmanError::InvalidSetup(format!(
                "ZiskExecutorTest: no manager registered for air_id {air_id}; \
                 either the SM is not in build_sm_bundle or its entry is missing \
                 from build_manager_registry"
            ))
        })
    }

    /// Replace the registered hook bag. Cleared automatically on the next
    /// `execute()`; pass `UnitTestHookBag::new()` to remove all hooks.
    pub fn set_hooks(&self, hooks: UnitTestHookBag) {
        *self.hooks.write().unwrap() = hooks;
    }

    /// Replace the registered trace-override bag. For any AIR id with an
    /// override, `calculate_witness` builds the `AirInstance` from the
    /// override closure instead of running the SM's `compute_witness`. Pass
    /// `TraceOverrideBag::new()` to remove all overrides.
    pub fn set_trace_overrides(&self, overrides: TraceOverrideBag) {
        *self.trace_overrides.write().unwrap() = overrides;
    }

    /// Set the type-erased input data. This is the only input path: the typed
    /// `verify_input()` builder boxes each input into this map before running.
    pub fn set_input_data_value(&self, value: HashMap<String, Vec<Box<dyn Any + Send + Sync>>>) {
        *self.input_data_value.write().unwrap() = Some(value);
    }

    /// Switches between packed and non-packed trace-row layouts. GPU mode
    /// in proofman implies packed; the unit-test backend forwards the bit.
    pub fn set_packed(&self, packed: bool) {
        self.packed.store(packed, Ordering::SeqCst);
    }

    /// Take the input map set by [`Self::set_input_data_value`]. The boxed
    /// inputs are not `Clone`, so this consumes the stored map (one `execute`
    /// per `set`).
    fn take_input_map(&self) -> ProofmanResult<HashMap<String, Vec<Box<dyn Any + Send + Sync>>>> {
        self.input_data_value.write().unwrap().take().ok_or_else(|| {
            ProofmanError::InvalidSetup("ZiskExecutorTest: input_data_value not set".into())
        })
    }
}

impl WitnessComponent<Goldilocks> for ZiskExecutorTest {
    fn execute(
        &self,
        pctx: Arc<ProofCtx<Goldilocks>>,
        _sctx: Arc<SetupCtx<Goldilocks>>,
        global_ids: &RwLock<Vec<usize>>,
    ) -> ProofmanResult<()> {
        let raw = self.take_input_map()?;

        // Drive the registry: for every SM-name key, look up the matching SM,
        // collect its already-typed (erased) inputs, and let the SM plan its
        // own AIR instances. The unit-test executor itself is fully generic
        // here; per-SM behaviour lives entirely in trait impls.
        for (key, arr) in raw {
            let sm = lookup_by_name(&key).ok_or_else(|| {
                ProofmanError::InvalidSetup(format!(
                    "ZiskExecutorTest: unknown state-machine name `{key}`"
                ))
            })?;
            let typed_inputs = sm.collect_inputs(arr)?;
            let mgr = self.manager_for(sm.air_id())?;
            let chunks = sm.plan_erased(mgr, &pctx, typed_inputs)?;
            for (gid, chunk) in chunks {
                global_ids.write().unwrap().push(gid);
                self.air_ids.write().unwrap().insert(gid, sm.air_id());
                self.inputs.write().unwrap().insert(gid, chunk);
            }
        }

        Ok(())
    }

    fn calculate_witness(
        &self,
        stage: u32,
        pctx: Arc<ProofCtx<Goldilocks>>,
        sctx: Arc<SetupCtx<Goldilocks>>,
        instance_ids: &[usize],
        n_cores: usize,
        buffer_pool: &dyn BufferPool<Goldilocks>,
    ) -> ProofmanResult<()> {
        if stage != 1 {
            return Ok(());
        }

        let pool = create_pool(n_cores);
        let packed = self.packed.load(Ordering::SeqCst);

        let result: ProofmanResult<()> = pool.install(|| {
            for &global_id in instance_ids {
                let chunk = match self.inputs.write().unwrap().remove(&global_id) {
                    Some(c) => c,
                    None => continue,
                };
                let air_id = self
                    .air_ids
                    .read()
                    .unwrap()
                    .get(&global_id)
                    .copied()
                    .ok_or_else(|| {
                        ProofmanError::InvalidSetup(format!(
                            "ZiskExecutorTest: no air_id recorded for global_id {global_id}"
                        ))
                    })?;
                let sm = lookup_by_air_id(air_id).ok_or_else(|| {
                    ProofmanError::InvalidSetup(format!(
                        "ZiskExecutorTest: unsupported air_id {air_id}"
                    ))
                })?;
                let trace_buffer = buffer_pool.take_buffer();

                // Raw trace-authoring override: if registered for this AIR
                // id, bypass `compute_witness` and let the user closure write
                // the trace directly. Otherwise take the normal path.
                let mut air_instance = {
                    let overrides_guard = self.trace_overrides.read().unwrap();
                    if let Some(slot) = overrides_guard.overrides.get(&air_id) {
                        if packed {
                            return Err(ProofmanError::InvalidSetup(
                                "ZiskExecutorTest: trace overrides are not supported in packed (GPU) mode".into(),
                            ));
                        }
                        let builder = lookup_override_by_air_id(air_id).ok_or_else(|| {
                            ProofmanError::InvalidSetup(format!(
                                "ZiskExecutorTest: override registered for air_id {air_id} \
                                 but no DynTraceOverride builder is in OVERRIDE_REGISTRY"
                            ))
                        })?;
                        builder.build_erased(slot.override_fn.as_ref(), chunk, trace_buffer)?
                    } else {
                        drop(overrides_guard);
                        let mgr = self.manager_for(air_id)?;
                        sm.compute_witness_erased(mgr, &sctx, chunk, trace_buffer, packed)?
                    }
                };

                // Post-hoc trace-row injection: if a hook is registered
                // for this AIR id, walk the buffer one row at a time and
                // call the user closure. Hooks are typed; the cast back to
                // `&mut Row` happens inside the per-SM erased closure.
                {
                    let hooks_guard = self.hooks.read().unwrap();
                    if let Some(slot) = hooks_guard.hooks.get(&air_id) {
                        if packed {
                            return Err(ProofmanError::InvalidSetup(
                                "ZiskExecutorTest: trace-row hooks are not supported in packed (GPU) mode".into(),
                            ));
                        }
                        let buf = &mut air_instance.trace;
                        let n_rows = buf.len() / slot.row_size;
                        for row_idx in 0..n_rows {
                            let start = row_idx * slot.row_size;
                            let end = start + slot.row_size;
                            (slot.apply)(row_idx, &mut buf[start..end]);
                        }
                    }
                }

                pctx.add_air_instance(air_instance, global_id);
            }
            Ok(())
        });

        result
    }
}

impl ZiskExecutorTest {
    /// Erase to `Arc<dyn Any>` for storage in type-erased registries.
    pub fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

/// Sibling of the production executor initialization that registers a
/// [`ZiskExecutorTest`] instead of the production `ZiskExecutor`. Builds the
/// same [`StaticSMBundle`] (built-ins created internally + precompiles from
/// [`Precompiles::all`]) so the unit-test path stays in lockstep with
/// production. The test executor reads typed state-machine inputs (from JSON
/// or programmatically via the session API) and skips ROM execution.
///
/// Concrete to `Goldilocks` because the unit-test trait registry is
/// `Goldilocks`-only.
pub fn initialize_executor_test(
    verbose_mode: proofman_common::VerboseMode,
    shared_tables: bool,
    wcm: &WitnessManager<Goldilocks>,
) -> ProofmanResult<Arc<ZiskExecutorTest>> {
    let rank_info = wcm.get_rank_info();
    proofman_common::initialize_logger(verbose_mode, Some(&rank_info));

    let std = pil_std_lib::Std::new(wcm.get_pctx(), wcm.get_sctx(), shared_tables)?;
    proofman::register_std(wcm, &std);

    let precompiles = Precompiles::all(std.clone());
    let sm_bundle = StaticSMBundle::new(std, precompiles);

    let executor = Arc::new(ZiskExecutorTest::new(sm_bundle));
    wcm.register_component(executor.clone());
    wcm.set_witness_initialized();

    Ok(executor)
}
