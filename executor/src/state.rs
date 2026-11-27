//! Shared execution state for the ZisK executor components.

mod chunk_collector_store;
mod instance_set;

pub use chunk_collector_store::*;
pub use instance_set::*;

use arc_swap::ArcSwap;
use proofman_fields::PrimeField64;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, PoisonError, RwLock,
};
use zisk_common::{
    io::ZiskStdin, BusDevice, CollectMemoryReport, EmuTrace, ExecutorStatsHandle, InstanceType,
    RandomSeed, Stats, ZiskExecutorSummary,
};
use zisk_core::ZiskRom;

use crate::error::{ExecutorError, ExecutorResult, RwLockExt};

/// Type alias for chunk collectors: (chunk_id, collector)
pub type ChunkCollector = (usize, Box<dyn BusDevice<u64>>);

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

    /// Seed for the next run, set together with `stdin` by `set_stdin_and_random_seed`. `None`
    /// until the first call.
    random_seed: Mutex<Option<RandomSeed>>,

    /// Planning information for main state machines (minimal traces from emulation).
    /// `Arc<EmuTrace>` elements so the store can be filled progressively while the
    /// emulator streams chunks (main witness advancement) and readers can clone the
    /// cheap Arc vector instead of holding the lock across a witness computation.
    pub min_traces: Arc<RwLock<Option<Vec<Arc<EmuTrace>>>>>,

    /// Main + secondary instance maps populated by `PlanPhase`.
    pub instance_set: Arc<InstanceSet<F>>,

    /// Per-instance chunk collectors. Lock-contested during the
    /// witness phase.
    pub collector_store: Arc<ChunkCollectorStore>,

    /// Execution result, including the number of executed steps.
    pub execution_result: Mutex<ZiskExecutorSummary>,

    /// How the witness phase's collects fitted in memory. Apart from `execution_result`, which the
    /// execution sets whole when it ends, as most collects come after it.
    pub collect_memory: Mutex<CollectMemoryReport>,

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
            random_seed: Mutex::new(None),
            min_traces: Arc::new(RwLock::new(None)),
            instance_set: Arc::new(InstanceSet::new()),
            collector_store: Arc::new(ChunkCollectorStore::new()),
            execution_result: Mutex::new(ZiskExecutorSummary::default()),
            collect_memory: Mutex::new(CollectMemoryReport::default()),
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

    /// Sets the standard input and seed for the next execution.
    pub fn set_stdin_and_random_seed(&self, stdin: ZiskStdin, seed: RandomSeed) {
        self.stdin.store(Arc::new(stdin));
        *self.random_seed.lock().unwrap_or_else(PoisonError::into_inner) = Some(seed);
    }

    /// Gets a snapshot of the current standard input.
    pub fn get_stdin(&self) -> Arc<ZiskStdin> {
        self.stdin.load_full()
    }

    /// Gets the seed for the current execution.
    ///
    /// # Errors
    /// Returns [`ExecutorError::RandomSeedNotSet`] if `set_stdin_and_random_seed` has not been
    /// called yet.
    pub fn get_random_seed(&self) -> ExecutorResult<RandomSeed> {
        self.random_seed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .ok_or(ExecutorError::RandomSeedNotSet)
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
        *self.collect_memory.lock().unwrap_or_else(PoisonError::into_inner) =
            CollectMemoryReport::default();
        *self.min_traces.write().unwrap_or_else(PoisonError::into_inner) = None;
        self.instance_set.reset();
        self.collector_store.reset();
        self.stats.reset();
    }

    /// Gets a clone of the execution result.
    pub fn get_execution_result(&self) -> ZiskExecutorSummary {
        let mut result = self.execution_result.lock().unwrap().clone();
        result.collect_memory = *self.collect_memory.lock().unwrap_or_else(PoisonError::into_inner);
        result
    }

    /// Records instances the first registration round left for later, reserving `bytes`.
    pub fn record_deferred_collect(&self, instances: usize, bytes: u64) {
        let mut report = self.collect_memory.lock().unwrap_or_else(PoisonError::into_inner);
        report.deferred_instances += instances as u64;
        report.deferred_bytes += bytes;
    }

    /// Records a collect that ran in `waves`, waiting `waited` for room.
    pub fn record_split_collect(&self, waves: usize, waited: std::time::Duration) {
        let mut report = self.collect_memory.lock().unwrap_or_else(PoisonError::into_inner);
        report.split_collects += 1;
        report.waves += waves as u64;
        report.wave_wait_ms += waited.as_millis() as u64;
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

#[cfg(test)]
mod tests {
    use super::*;
    use proofman_fields::Goldilocks;

    type F = Goldilocks;

    const SEED: RandomSeed = RandomSeed::new([3; RandomSeed::LEN]);

    #[test]
    fn get_random_seed_fails_until_stdin_and_random_seed_are_set() {
        let state = ExecutionState::<F>::new();
        assert!(matches!(state.get_random_seed(), Err(ExecutorError::RandomSeedNotSet)));

        state.set_stdin_and_random_seed(ZiskStdin::new(), SEED);
        assert_eq!(state.get_random_seed().unwrap(), SEED);
    }

    /// Most collects run after the execution has set its summary whole: the report they record
    /// must survive that, and go with the next job's reset.
    #[test]
    fn the_collect_report_outlives_the_execution_summary_until_the_next_job() {
        let state = ExecutionState::<F>::new();
        state.record_deferred_collect(44, 9 << 30);
        state.set_execution_result(ZiskExecutorSummary { steps: 7, ..Default::default() });
        state.record_split_collect(5, std::time::Duration::from_millis(200));
        state.record_split_collect(3, std::time::Duration::from_millis(100));

        let result = state.get_execution_result();
        assert_eq!(result.steps, 7);
        assert_eq!(
            result.collect_memory,
            CollectMemoryReport {
                deferred_instances: 44,
                deferred_bytes: 9 << 30,
                split_collects: 2,
                waves: 8,
                wave_wait_ms: 300,
            }
        );
        assert!(result.collect_memory.constrained());

        state.reset();
        assert_eq!(state.get_execution_result().collect_memory, CollectMemoryReport::default());
    }

    /// `execute_inner` resets the state after the caller has set the job's stdin and seed, so
    /// the reset must leave the seed alone.
    #[test]
    fn reset_keeps_the_seed() {
        let state = ExecutionState::<F>::new();
        state.set_stdin_and_random_seed(ZiskStdin::new(), SEED);

        state.reset();
        assert_eq!(state.get_random_seed().unwrap(), SEED);
    }
}
