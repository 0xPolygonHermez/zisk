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

/// Per-SM batches of type-erased inputs (each entry a boxed `S::Input`),
/// keyed by SM name. Mirrors `ErasedInputs` on the prover-backend side.
type ErasedInputMap = HashMap<String, Vec<Box<dyn Any + Send + Sync>>>;

/// Post-hoc mutator for one AIR's flat `airvalues` buffer, applied to every
/// instance of that AIR (identified by the 0-based per-AIR instance index)
/// after its witness (and any row hooks) are done.
pub type AirValuesFn = Box<dyn Fn(usize, &mut Vec<Goldilocks>) + Send + Sync>;

/// Mutator for the proof-wide values, applied once per run through the
/// typed [`zisk_pil::ZiskProofValues`] view over `pctx`'s buffer.
pub type ProofValuesFn =
    Box<dyn for<'a> Fn(&mut zisk_pil::ZiskProofValues<'a, Goldilocks>) + Send + Sync>;

/// Alternative `WitnessComponent` used by the unit-test backend.
///
/// The struct is concrete to `Goldilocks` because the registry of per-SM
/// trait impls is concrete to `Goldilocks` and we never instantiate the
/// unit-test path with any other field. Keeping it concrete avoids a layer
/// of generic bounds that would otherwise have to be threaded through every
/// call site.
pub struct ZiskExecutorTest {
    sm_bundle: Arc<StaticSMBundle<Goldilocks>>,
    /// Per-AIR-id erased Manager arcs, built once from the bundle.
    managers: HashMap<usize, Arc<dyn Any + Send + Sync>>,
    /// Per-instance erased input chunks, keyed by global_id.
    inputs: RwLock<HashMap<usize, Box<dyn Any + Send + Sync>>>,
    /// Per-instance AIR id, recorded at planning time.
    air_ids: RwLock<HashMap<usize, usize>>,
    /// Type-erased input data set by the `VerifyInput` builder before each
    /// run. The only input source for the unit-test executor.
    input_data_value: RwLock<Option<ErasedInputMap>>,
    /// Per-AIR-id post-hoc trace-row hooks ([`ZiskExecutorTest::set_hooks`]).
    hooks: RwLock<UnitTestHookBag>,
    /// Per-AIR-id raw trace-authoring overrides
    /// ([`ZiskExecutorTest::set_trace_overrides`]); an override bypasses the
    /// SM's `compute_witness` for that AIR id.
    trace_overrides: RwLock<TraceOverrideBag>,
    /// Per-AIR-id air-values mutators ([`ZiskExecutorTest::set_air_values_hooks`]).
    air_values_hooks: RwLock<HashMap<usize, AirValuesFn>>,
    /// Proof-values mutator ([`ZiskExecutorTest::set_proof_values_fn`]).
    proof_values_fn: RwLock<Option<ProofValuesFn>>,
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
            air_values_hooks: RwLock::new(HashMap::new()),
            proof_values_fn: RwLock::new(None),
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
    /// `VerifyInput` builder boxes each input into this map before running.
    pub fn set_input_data_value(&self, value: ErasedInputMap) {
        *self.input_data_value.write().unwrap() = Some(value);
    }

    /// Replace the per-AIR-id air-values mutators. Each closure runs against
    /// the flat `airvalues` of every instance of its AIR, after the witness
    /// and any row hooks. Pass an empty map to remove them all.
    pub fn set_air_values_hooks(&self, hooks: HashMap<usize, AirValuesFn>) {
        *self.air_values_hooks.write().unwrap() = hooks;
    }

    /// Replace the proof-values mutator, run once per `execute()` against the
    /// typed `ZiskProofValues` view. Pass `None` to remove it.
    pub fn set_proof_values_fn(&self, f: Option<ProofValuesFn>) {
        *self.proof_values_fn.write().unwrap() = f;
    }

    /// Switches between packed and non-packed trace-row layouts. GPU mode
    /// in proofman implies packed; the unit-test backend forwards the bit.
    pub fn set_packed(&self, packed: bool) {
        self.packed.store(packed, Ordering::SeqCst);
    }

    /// Take the input map set by [`Self::set_input_data_value`]. The boxed
    /// inputs are not `Clone`, so this consumes the stored map (one `execute`
    /// per `set`). A run that only registers trace-authoring overrides has no
    /// inputs to set, so a missing map is fine in that case.
    fn take_input_map(&self) -> ProofmanResult<ErasedInputMap> {
        match self.input_data_value.write().unwrap().take() {
            Some(map) => Ok(map),
            None if !self.trace_overrides.read().unwrap().is_empty() => Ok(HashMap::new()),
            None => Err(ProofmanError::InvalidSetup(
                "ZiskExecutorTest: input_data_value not set".into(),
            )),
        }
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

        // Proof-wide values first: they are reset by the proofman before
        // every run, so this is the earliest seam where writes survive.
        if let Some(f) = self.proof_values_fn.read().unwrap().as_ref() {
            let mut pv = zisk_pil::ZiskProofValues::from_vec_guard(pctx.get_proof_values());
            f(&mut pv);
        }

        // Drive the registry: for every SM-name key, look up the matching SM,
        // collect its already-typed (erased) inputs, and let the SM plan its
        // own AIR instances. The unit-test executor itself is fully generic
        // here; per-SM behaviour lives entirely in trait impls.
        let mut planned_air_ids = std::collections::HashSet::new();
        for (key, arr) in raw {
            let sm = lookup_by_name(&key).ok_or_else(|| {
                ProofmanError::InvalidSetup(format!(
                    "ZiskExecutorTest: unknown state-machine name `{key}`"
                ))
            })?;
            let typed_inputs = sm.collect_inputs(arr)?;
            let mgr = self.manager_for(sm.air_id())?;
            let chunks = sm.plan_erased(mgr, &pctx, typed_inputs)?;
            planned_air_ids.insert(sm.air_id());
            for (gid, chunk) in chunks {
                global_ids.write().unwrap().push(gid);
                self.air_ids.write().unwrap().insert(gid, sm.air_id());
                self.inputs.write().unwrap().insert(gid, chunk);
            }
        }

        // Shape-driven planning for trace-authoring overrides: `trace()`
        // takes no inputs, so an override whose AIR id was not planned from
        // inputs gets its requested number of instances here (each sized to
        // the AIR's fixed NUM_ROWS — nothing input-dependent to decide).
        for (&air_id, slot) in &self.trace_overrides.read().unwrap().overrides {
            if planned_air_ids.contains(&air_id) {
                continue;
            }
            for _ in 0..slot.instances {
                let gid = pctx.add_instance(zisk_pil::ZISK_AIRGROUP_ID, air_id)?;
                global_ids.write().unwrap().push(gid);
                self.air_ids.write().unwrap().insert(gid, air_id);
                // Marker entry so `calculate_witness` picks the instance up;
                // the override path never reads it.
                self.inputs.write().unwrap().insert(gid, Box::new(()));
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
                // 0-based instance index within this AIR — lets multi-instance
                // hooks/overrides tell instances apart (e.g. segment chains).
                let instance_idx = pctx.dctx_find_air_instance_id(global_id)?;

                // Raw trace authoring: an override bypasses `compute_witness`
                // and lets the user closure write the trace directly.
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
                        builder.build_erased(
                            slot.override_fn.as_ref(),
                            &self.sm_bundle.get_std(),
                            trace_buffer,
                            instance_idx,
                        )?
                    } else {
                        drop(overrides_guard);
                        let mgr = self.manager_for(air_id)?;
                        sm.compute_witness_erased(mgr, &sctx, chunk, trace_buffer, packed)?
                    }
                };

                // Post-hoc trace-row injection: walk the buffer one row at a
                // time and call the registered hooks in order.
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
                            let row = &mut buf[start..start + slot.row_size];
                            for hook in &slot.hooks {
                                hook(row_idx, row);
                            }
                        }
                    }
                }

                // Air-values injection: runs last, so it sees (and can fix up
                // or corrupt) whatever the witness path produced. For authored
                // traces the buffer starts empty — fill it from the AIR's
                // generated `<Air>AirValues::new()` + `get_buffer()`.
                if let Some(f) = self.air_values_hooks.read().unwrap().get(&air_id) {
                    f(instance_idx, &mut air_instance.airvalues);
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
