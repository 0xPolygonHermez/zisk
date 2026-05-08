use std::{
    any::Any,
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use fields::Goldilocks;
use proofman_common::{create_pool, BufferPool, ProofCtx, ProofmanError, ProofmanResult, SetupCtx};
use witness::WitnessComponent;

use crate::{
    unit_test_hooks::UnitTestHookBag,
    unit_test_targets::{build_manager_registry, lookup_by_air_id, lookup_by_name},
    StaticSMBundle,
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
    /// Path to the JSON input file. Set by the CLI / test harness before
    /// `proofman.verify_proof_constraints_from_lib` triggers `execute()`.
    input_data_path: RwLock<Option<PathBuf>>,
    /// Pre-parsed input data, used by the programmatic session API
    /// (avoids serialise → write tempfile → read → deserialise round-trip).
    input_data_value: RwLock<Option<HashMap<String, Vec<serde_json::Value>>>>,
    /// Per-AIR-id post-hoc trace-row hooks. Empty by default; set via
    /// [`ZiskExecutorTest::set_hooks`]. Cleared at the start of every
    /// `execute()` so leftover hooks from a previous run don't apply.
    hooks: RwLock<UnitTestHookBag>,
    packed: AtomicBool,
}

impl ZiskExecutorTest {
    pub fn new(sm_bundle: StaticSMBundle<Goldilocks>) -> Self {
        let managers = build_manager_registry(&sm_bundle);
        Self {
            sm_bundle: Arc::new(sm_bundle),
            managers,
            inputs: RwLock::new(HashMap::new()),
            air_ids: RwLock::new(HashMap::new()),
            input_data_path: RwLock::new(None),
            input_data_value: RwLock::new(None),
            hooks: RwLock::new(UnitTestHookBag::new()),
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

    /// Set the JSON file path that `execute()` will read on the next run.
    /// Mutually exclusive with `set_input_data_value`; whichever is set
    /// last wins.
    pub fn set_input_data_path(&self, path: PathBuf) {
        *self.input_data_path.write().unwrap() = Some(path);
        *self.input_data_value.write().unwrap() = None;
    }

    /// Set the input data directly as a parsed JSON map. Used by the
    /// programmatic `UnitTestSession` API.
    pub fn set_input_data_value(&self, value: HashMap<String, Vec<serde_json::Value>>) {
        *self.input_data_value.write().unwrap() = Some(value);
        *self.input_data_path.write().unwrap() = None;
    }

    /// Switches between packed and non-packed trace-row layouts. GPU mode
    /// in proofman implies packed; the unit-test backend forwards the bit.
    pub fn set_packed(&self, packed: bool) {
        self.packed.store(packed, Ordering::SeqCst);
    }

    /// Read the JSON map either from the configured file path or from the
    /// pre-parsed value, whichever is set.
    fn load_input_map(&self) -> ProofmanResult<HashMap<String, Vec<serde_json::Value>>> {
        if let Some(value) = self.input_data_value.read().unwrap().clone() {
            return Ok(value);
        }
        let path = self.input_data_path.read().unwrap().clone().ok_or_else(|| {
            ProofmanError::InvalidSetup(
                "ZiskExecutorTest: neither input_data_path nor input_data_value set".into(),
            )
        })?;
        let json_text = fs::read_to_string(&path).map_err(|e| {
            ProofmanError::InvalidSetup(format!(
                "ZiskExecutorTest: failed to read input JSON {}: {}",
                path.display(),
                e
            ))
        })?;
        serde_json::from_str(&json_text).map_err(|e| {
            ProofmanError::InvalidSetup(format!("ZiskExecutorTest: invalid JSON: {e}"))
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
        let raw = self.load_input_map()?;

        // Drive the registry: for every JSON top-level key, look up the
        // matching SM, deserialise the inputs, and let the SM plan its own
        // AIR instances. The unit-test executor itself is fully generic
        // here; per-SM behaviour lives entirely in trait impls.
        for (key, arr) in raw {
            let sm = lookup_by_name(&key).ok_or_else(|| {
                ProofmanError::InvalidSetup(format!(
                    "ZiskExecutorTest: unknown state-machine name `{key}`"
                ))
            })?;
            let typed_inputs = sm.deserialize_inputs(arr)?;
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
                let mgr = self.manager_for(air_id)?;
                let mut air_instance =
                    sm.compute_witness_erased(mgr, &sctx, chunk, trace_buffer, packed)?;

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
    pub fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}
