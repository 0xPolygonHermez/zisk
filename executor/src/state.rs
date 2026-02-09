//! Shared execution state for the ZisK executor components.

use fields::PrimeField64;
use sm_main::MainInstance;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
use zisk_common::{BusDevice, EmuTrace, ExecutorStatsHandle, Instance, Plan, ZiskExecutionResult};
use zisk_core::ZiskRom;

/// Type alias for chunk collectors: (chunk_id, collector)
pub type ChunkCollector = (usize, Box<dyn BusDevice<u64>>);

pub struct ExecutionState<F: PrimeField64> {
    /// ZisK ROM (ELF), can be changed between executions.
    pub zisk_rom: RwLock<Option<Arc<ZiskRom>>>,

    /// Planning information for main state machines (minimal traces from emulation).
    pub min_traces: Arc<RwLock<Option<Vec<EmuTrace>>>>,

    /// Planning information for secondary state machines.
    pub secn_planning: RwLock<Vec<Plan>>,

    /// Main state machine instances, indexed by their global ID.
    pub main_instances: RwLock<HashMap<usize, MainInstance<F>>>,

    /// Secondary state machine instances, indexed by their global ID.
    pub secn_instances: RwLock<HashMap<usize, Box<dyn Instance<F>>>>,

    /// Collectors by instance, storing statistics and collectors for each instance.
    pub collectors_by_instance: Arc<RwLock<HashMap<usize, Vec<Option<ChunkCollector>>>>>,

    /// Execution result, including the number of executed steps.
    pub execution_result: Mutex<ZiskExecutionResult>,

    /// Statistics collected during the execution.
    pub stats: ExecutorStatsHandle,
}

impl<F: PrimeField64> ExecutionState<F> {
    /// Creates a new `ExecutionState` with default values.
    pub fn new() -> Self {
        Self {
            zisk_rom: RwLock::new(None),
            min_traces: Arc::new(RwLock::new(None)),
            secn_planning: RwLock::new(Vec::new()),
            main_instances: RwLock::new(HashMap::new()),
            secn_instances: RwLock::new(HashMap::new()),
            collectors_by_instance: Arc::new(RwLock::new(HashMap::new())),
            execution_result: Mutex::new(ZiskExecutionResult::default()),
            stats: ExecutorStatsHandle::new(),
        }
    }

    /// Sets the ZisK ROM for execution.
    ///
    /// This can be called between executions to change the ROM/ELF
    /// without recreating the executor.
    pub fn set_rom(&self, rom: Arc<ZiskRom>) {
        *self.zisk_rom.write().unwrap() = Some(rom);
    }

    /// Gets the current ZisK ROM.
    ///
    /// # Panics
    /// Panics if no ROM has been set.
    pub fn get_rom(&self) -> Arc<ZiskRom> {
        self.zisk_rom
            .read()
            .unwrap()
            .as_ref()
            .expect("ROM not set. Call set_rom() before execute()")
            .clone()
    }

    /// Resets all internal state to default values.
    pub fn reset(&self) {
        *self.execution_result.lock().unwrap() = ZiskExecutionResult::default();
        *self.min_traces.write().unwrap() = None;
        *self.secn_planning.write().unwrap() = Vec::new();
        self.main_instances.write().unwrap().clear();
        self.secn_instances.write().unwrap().clear();
        self.collectors_by_instance.write().unwrap().clear();
        self.stats.reset();
    }

    /// Gets a clone of the execution result.
    pub fn get_execution_result(&self) -> ZiskExecutionResult {
        self.execution_result.lock().unwrap().clone()
    }

    /// Sets the execution result.
    pub fn set_execution_result(&self, result: ZiskExecutionResult) {
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
