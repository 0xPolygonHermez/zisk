//! Shared execution state for the ZisK executor components.

mod chunk_collector_store;
mod instance_set;

pub use chunk_collector_store::*;
pub use instance_set::*;

use arc_swap::ArcSwap;
use proofman_fields::PrimeField64;
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, PoisonError, RwLock,
};
use zisk_common::{
    io::ZiskStdin, BusDevice, EmuTrace, ExecutorStatsHandle, InstCount, Instance, InstanceType,
    PrecompileLogs, Stats, ZiskExecutorSummary,
};
use zisk_core::ZiskRom;

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};

/// Type alias for chunk collectors: (chunk_id, collector)
pub type ChunkCollector = (usize, Box<dyn BusDevice<u64>>);

/// An instance's `(chunk, operations, skipped)`, sorted by chunk.
pub type ChunkOps = Vec<(usize, u64, u64)>;

/// One air's instances planned from the log.
#[derive(Default)]
pub struct EarlyAir {
    /// Operations per chunk so far.
    pub counts: Vec<InstCount>,
    /// Each planned instance, in order.
    pub planned: Vec<ChunkOps>,
}

/// The precompile instances planned from the logs during the run.
#[derive(Default)]
pub struct EarlySecn {
    /// By `(airgroup_id, air_id)`.
    pub airs: HashMap<(usize, usize), EarlyAir>,
    /// Already announced.
    pub announced: HashSet<usize>,
}

/// Execution state for the ZisK executor.
///
/// The instance maps and chunk-collector map live behind dedicated
/// wrappers ([`InstanceSet`] / [`ChunkCollectorStore`]). Their access
/// paths get one extra hop (`state.instance_set.main_instances`
/// instead of `state.main_instances`) but the names document the
/// lifecycle: `InstanceSet` is *write-once* in [`crate::PlanPhase`],
/// `ChunkCollectorStore` is *lock-contested* during witness collection.
pub struct ExecutionState<F: PrimeField64> {
    /// ZisK ROM (ELF), can be changed between executions.
    pub zisk_rom: RwLock<Option<Arc<ZiskRom>>>,

    /// Standard input for the next run. Bridges the caller-set value and
    /// the framework-driven `WitnessComponent::execute` (whose trait
    /// signature has no stdin slot).
    pub stdin: ArcSwap<ZiskStdin>,

    /// Planning information for main state machines (minimal traces from emulation).
    /// `Arc<EmuTrace>` elements so the store can be filled progressively while the
    /// emulator streams chunks (main witness advancement) and readers can clone the
    /// cheap Arc vector instead of holding the lock across a witness computation.
    pub min_traces: Arc<RwLock<Option<Vec<Arc<EmuTrace>>>>>,

    /// The precompile logs of the run.
    pub precompile_logs: RwLock<Arc<PrecompileLogs>>,

    /// Precompile instances released during the run.
    pub early_secn: Mutex<EarlySecn>,

    /// Main + secondary instance maps populated by `PlanPhase`.
    pub instance_set: Arc<InstanceSet<F>>,

    /// Per-instance chunk collectors. Lock-contested during the
    /// witness phase.
    pub collector_store: Arc<ChunkCollectorStore>,

    /// Execution result, including the number of executed steps.
    pub execution_result: Mutex<ZiskExecutorSummary>,

    /// Statistics collected during the execution.
    pub stats: ExecutorStatsHandle,

    /// Flag to indicate whether to use hints during execution
    pub use_hints: AtomicBool,
}

impl<F: PrimeField64> ExecutionState<F> {
    /// Creates a new `ExecutionState` with default values.
    pub fn new() -> Self {
        Self {
            zisk_rom: RwLock::new(None),
            stdin: ArcSwap::from_pointee(ZiskStdin::new()),
            min_traces: Arc::new(RwLock::new(None)),
            precompile_logs: RwLock::new(Arc::default()),
            early_secn: Mutex::new(EarlySecn::default()),
            instance_set: Arc::new(InstanceSet::new()),
            collector_store: Arc::new(ChunkCollectorStore::new()),
            execution_result: Mutex::new(ZiskExecutorSummary::default()),
            stats: ExecutorStatsHandle::new(),
            use_hints: AtomicBool::new(false),
        }
    }

    /// Sets the ZisK ROM for execution.
    ///
    /// This can be called between executions to change the ROM/ELF
    /// without recreating the executor.
    pub fn set_rom(&self, rom: Arc<ZiskRom>, use_hints: bool) {
        *self.zisk_rom.write().unwrap() = Some(rom);
        self.use_hints.store(use_hints, Ordering::SeqCst);
    }

    /// Gets the current ZisK ROM.
    ///
    /// # Errors
    /// Returns an error if no ROM has been set via `set_rom()` or if the ROM lock is poisoned.
    pub fn get_rom(&self) -> ExecutorResult<Arc<ZiskRom>> {
        let guard = self.zisk_rom.read_or_poison("rom")?;
        guard.as_ref().cloned().ok_or(ExecutorError::RomNotInitialized)
    }

    /// Sets the standard input for the next execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) {
        self.stdin.store(Arc::new(stdin));
    }

    /// Gets a snapshot of the current standard input.
    pub fn get_stdin(&self) -> Arc<ZiskStdin> {
        self.stdin.load_full()
    }

    /// Resets all internal state to default values.
    ///
    /// Poison-tolerant: every lock here is unwrapped via
    /// `PoisonError::into_inner` so a prior-execution panic does not
    /// cascade and leave later fields un-reset. Sound only because each
    /// lock's contents is overwritten — do NOT copy this pattern to
    /// non-reset call sites.
    pub fn reset(&self) {
        *self.execution_result.lock().unwrap_or_else(PoisonError::into_inner) =
            ZiskExecutorSummary::default();
        *self.min_traces.write().unwrap_or_else(PoisonError::into_inner) = None;
        *self.precompile_logs.write().unwrap_or_else(PoisonError::into_inner) = Arc::default();
        *self.early_secn.lock().unwrap_or_else(PoisonError::into_inner) = EarlySecn::default();
        self.instance_set.reset();
        self.collector_store.reset();
        self.stats.reset();
    }

    /// Gets a clone of the execution result.
    pub fn get_execution_result(&self) -> ZiskExecutorSummary {
        self.execution_result.lock().unwrap().clone()
    }

    /// Sets the execution result.
    pub fn set_execution_result(&self, result: ZiskExecutorSummary) {
        *self.execution_result.lock().unwrap() = result;
    }

    /// Gets a clone of the stats handle.
    pub fn get_stats(&self) -> ExecutorStatsHandle {
        self.stats.clone()
    }

    /// Drains the per-chunk collectors recorded for `global_id` from
    /// `state.collector_store`. Returns an empty list when the instance
    /// is a `Table` (tables don't have per-chunk collectors).
    ///
    /// # Errors
    /// * `Instance`: errors if the global_id has no recorded entry, or if
    ///   any chunk slot is `None`.
    #[allow(clippy::type_complexity)]
    pub(super) fn take_collectors_for_instance(
        &self,
        global_id: usize,
        instance_type: InstanceType,
    ) -> ExecutorResult<Vec<(usize, Box<dyn BusDevice<u64>>)>> {
        match instance_type {
            InstanceType::Instance => {
                let mut guard = self.collector_store.inner.write_or_poison("collector_store")?;

                let collectors =
                    guard.remove(&global_id).ok_or(ExecutorError::MissingIndexEntry {
                        global_id,
                        index: "collector_store",
                    })?;

                collectors
                    .into_iter()
                    .enumerate()
                    .map(|(idx, opt)| {
                        opt.ok_or_else(|| {
                            ExecutorError::Internal(format!(
                                "collector at index {idx} for global_id {global_id} is None"
                            ))
                        })
                    })
                    .collect::<ExecutorResult<Vec<_>>>()
            }
            InstanceType::Table => Ok(vec![]),
        }
    }

    /// Fills an instance's collectors from the logs; false if it cannot be built from them.
    pub(crate) fn register_log_collectors(
        &self,
        global_id: usize,
        (airgroup_id, air_id): (usize, usize),
        instance: &dyn Instance<F>,
        logs: &PrecompileLogs,
        traces: &[Arc<EmuTrace>],
    ) -> ExecutorResult<bool> {
        let Some(collectors) = instance.collectors_from_log(logs, traces) else {
            return Ok(false);
        };
        self.collector_store
            .inner
            .write_or_poison("collector_store")?
            .insert(global_id, collectors.into_iter().map(Some).collect());
        self.stats.insert_witness_stats(global_id, Stats::new_no_collection(airgroup_id, air_id));
        Ok(true)
    }

    /// [`Self::register_log_collectors`] from the stored logs.
    pub(crate) fn register_stored_log_collectors(
        &self,
        global_id: usize,
        air: (usize, usize),
        instance: &dyn Instance<F>,
    ) -> ExecutorResult<bool> {
        let logs = self.precompile_logs.read_or_poison("precompile_logs")?.clone();
        let min_traces = self.min_traces.read_or_poison("min_traces")?;
        let Some(traces) = min_traces.as_ref() else { return Ok(false) };
        self.register_log_collectors(global_id, air, instance, &logs, traces)
    }

    /// Records an empty per-chunk collector slot for an instance that
    /// skips per-chunk collection (today: the ASM ROM path). Also pins a
    /// `Stats::new_no_collection` entry so observability reflects the
    /// "skipped collection" state.
    pub(crate) fn register_empty_collector(
        &self,
        global_id: usize,
        airgroup_id: usize,
        air_id: usize,
    ) -> ExecutorResult<()> {
        let stats = Stats::new_no_collection(airgroup_id, air_id);

        self.collector_store
            .inner
            .write_or_poison("collector_store")?
            .insert(global_id, Vec::new());
        self.stats.insert_witness_stats(global_id, stats);

        Ok(())
    }
}

impl<F: PrimeField64> Default for ExecutionState<F> {
    fn default() -> Self {
        Self::new()
    }
}
