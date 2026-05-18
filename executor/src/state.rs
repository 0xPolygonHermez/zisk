//! Shared execution state for the ZisK executor components.

use fields::PrimeField64;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, PoisonError, RwLock,
};
use zisk_common::{BusDevice, EmuTrace, ExecutorStatsHandle, ZiskExecutorSummary};
use zisk_core::ZiskRom;

use anyhow::Result;

use crate::{ChunkCollectorStore, InstanceSet};

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

    /// Planning information for main state machines (minimal traces from emulation).
    pub min_traces: Arc<RwLock<Option<Vec<EmuTrace>>>>,

    /// Main + secondary instance maps populated by `PlanPhase`.
    pub instance_set: Arc<InstanceSet<F>>,

    /// Per-instance chunk collectors. Lock-contested during the
    /// witness phase.
    pub collector_store: Arc<ChunkCollectorStore>,

    /// Execution result, including the number of executed steps.
    pub execution_result: Mutex<ZiskExecutorSummary>,

    /// Statistics collected during the execution.
    pub stats: ExecutorStatsHandle,

    /// Flag to indicate if the ROM has been initialized
    pub is_rom_initialized: AtomicBool,

    /// Flag to indicate whether to use hints during execution
    pub use_hints: AtomicBool,
}

impl<F: PrimeField64> ExecutionState<F> {
    /// Creates a new `ExecutionState` with default values.
    pub fn new() -> Self {
        Self {
            zisk_rom: RwLock::new(None),
            min_traces: Arc::new(RwLock::new(None)),
            instance_set: Arc::new(InstanceSet::new()),
            collector_store: Arc::new(ChunkCollectorStore::new()),
            execution_result: Mutex::new(ZiskExecutorSummary::default()),
            stats: ExecutorStatsHandle::new(),
            is_rom_initialized: AtomicBool::new(false),
            use_hints: AtomicBool::new(false),
        }
    }

    /// Sets the ZisK ROM for execution.
    ///
    /// This can be called between executions to change the ROM/ELF
    /// without recreating the executor.
    pub fn set_rom(&self, rom: Arc<ZiskRom>, use_hints: bool) {
        *self.zisk_rom.write().unwrap() = Some(rom);
        self.is_rom_initialized.store(true, Ordering::SeqCst);
        self.use_hints.store(use_hints, Ordering::SeqCst);
    }

    /// Gets the current ZisK ROM.
    ///
    /// # Errors
    /// Returns an error if no ROM has been set via `set_rom()` or if the ROM lock is poisoned.
    pub fn get_rom(&self) -> Result<Arc<ZiskRom>> {
        if !self.is_rom_initialized.load(Ordering::SeqCst) {
            anyhow::bail!("ROM not initialized. Call set_rom() before execute()");
        }

        let guard = self.zisk_rom.read().map_err(|e| anyhow::anyhow!("ROM lock poisoned: {e}"))?;

        guard.as_ref().cloned().ok_or_else(|| anyhow::anyhow!("ROM not initialized"))
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
}

impl<F: PrimeField64> Default for ExecutionState<F> {
    fn default() -> Self {
        Self::new()
    }
}
