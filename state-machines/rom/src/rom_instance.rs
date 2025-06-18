//! The `RomInstance` performs the witness computation based on the provided ROM execution plan
//!
//! It is responsible for computing witnesses for ROM-related execution plans,

use std::{
    collections::VecDeque,
    sync::{atomic::AtomicU32, Arc},
};

use crate::{rom_asm_worker::RomAsmWorker, rom_counter::RomCounter, RomSM};
use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use std::sync::Mutex;
use zisk_common::{
    create_atomic_vec, BusDevice, BusId, CheckPoint, ChunkId, CounterStats, Instance, InstanceCtx,
    InstanceType, Metrics, PayloadType, ROM_BUS_ID,
};
use zisk_core::ZiskRom;

/// The `RomInstance` struct represents an instance to perform the witness computations for
/// ROM-related execution plans.
///
/// It encapsulates the `ZiskRom` and its associated context, and it interacts with
/// the `RomSM` to compute witnesses for the given execution plan.
pub struct RomInstance {
    /// Reference to the Zisk ROM.
    zisk_rom: Arc<ZiskRom>,

    /// The instance context.
    ictx: InstanceCtx,

    /// Shared biod instruction counter for monitoring ROM operations.
    bios_inst_count: Mutex<Arc<Vec<AtomicU32>>>,

    /// Shared program instruction counter for monitoring ROM operations.
    prog_inst_count: Mutex<Arc<Vec<AtomicU32>>>,

    /// Execution statistics counter for ROM instructions.
    counter_stats: Mutex<Option<CounterStats>>,

    /// Optional worker for ROM assembly execution.
    rom_asm_worker: Mutex<Option<RomAsmWorker>>,
}

impl RomInstance {
    /// Creates a new `RomInstance`.
    ///
    /// # Arguments
    /// * `zisk_rom` - An `Arc`-wrapped reference to the Zisk ROM.
    /// * `ictx` - The `InstanceCtx` associated with this instance.
    ///
    /// # Returns
    /// A new `RomInstance` instance initialized with the provided ROM and context.
    pub fn new(
        zisk_rom: Arc<ZiskRom>,
        ictx: InstanceCtx,
        bios_inst_count: Arc<Vec<AtomicU32>>,
        prog_inst_count: Arc<Vec<AtomicU32>>,
        rom_asm_worker: Option<RomAsmWorker>,
    ) -> Self {
        Self {
            zisk_rom,
            ictx,
            bios_inst_count: Mutex::new(bios_inst_count),
            prog_inst_count: Mutex::new(prog_inst_count),
            counter_stats: Mutex::new(None),
            rom_asm_worker: Mutex::new(rom_asm_worker),
        }
    }

    pub fn is_asm_execution(&self) -> bool {
        self.rom_asm_worker.lock().unwrap().is_some()
    }
}

impl<F: PrimeField64> Instance<F> for RomInstance {
    /// Computes the witness for the ROM execution plan.
    ///
    /// This method leverages the `RomSM` to generate an `AirInstance` based on the
    /// Zisk ROM and the provided execution plan.
    ///
    /// # Arguments
    /// * `_pctx` - The proof context, unused in this implementation.
    /// * `_sctx` - The setup context, unused in this implementation.
    /// * `_collectors` - A vector of input collectors to process and collect data for witness,
    ///   unused in this implementation.
    ///
    /// # Returns
    /// An `Option` containing the computed `AirInstance`.
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        if self.is_asm_execution() {
            // Case 1: Use ROM assembly output
            let mut worker = self.rom_asm_worker.lock().unwrap().take().unwrap();
            let asm_runner_romh = worker.wait_for_task();

            *self.bios_inst_count.lock().unwrap() =
                Arc::new(create_atomic_vec(asm_runner_romh.asm_rowh_output.bios_inst_count.len()));
            *self.prog_inst_count.lock().unwrap() =
                Arc::new(create_atomic_vec(asm_runner_romh.asm_rowh_output.prog_inst_count.len()));

            return Some(RomSM::compute_witness_from_asm(
                &self.zisk_rom,
                &asm_runner_romh.asm_rowh_output,
            ));
        }

        // Case 2: Fallback to counter stats when not using assembly
        // Detach collectors and downcast to RomCollector
        if self.counter_stats.lock().unwrap().is_none() {
            let collectors: Vec<_> = collectors
                .into_iter()
                .map(|(_, collector)| collector.as_any().downcast::<RomCollector>().unwrap())
                .collect();

            let mut counter_stats = CounterStats::new(
                self.bios_inst_count.lock().unwrap().clone(),
                self.prog_inst_count.lock().unwrap().clone(),
            );

            for collector in collectors {
                counter_stats += &collector.rom_counter.counter_stats;
            }

            *self.counter_stats.lock().unwrap() = Some(counter_stats);
        }

        Some(RomSM::compute_witness(
            &self.zisk_rom,
            self.counter_stats.lock().unwrap().as_ref().unwrap(),
        ))
    }

    /// Retrieves the checkpoint associated with this instance.
    ///
    /// # Returns
    /// A `CheckPoint` object representing the checkpoint of the execution plan.
    fn check_point(&self) -> CheckPoint {
        self.ictx.plan.check_point.clone()
    }

    /// Retrieves the type of this instance.
    ///
    /// # Returns
    /// An `InstanceType` representing the type of this instance (`InstanceType::Instance`).
    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    /// Builds an input collector for the instance.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID associated with the input collector.
    ///
    /// # Returns
    /// An `Option` containing the input collector for the instance.
    fn build_inputs_collector(&self, _: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        if self.is_asm_execution() || self.counter_stats.lock().unwrap().is_some() {
            return None;
        }

        Some(Box::new(RomCollector::new(
            self.counter_stats.lock().unwrap().is_some(),
            self.bios_inst_count.lock().unwrap().clone(),
            self.prog_inst_count.lock().unwrap().clone(),
        )))
    }
}

pub struct RomCollector {
    /// Flag indicating if the table has been already computed.
    already_computed: bool,

    /// Execution statistics counter for the ROM.
    pub rom_counter: RomCounter,
}

impl RomCollector {
    /// Creates a new instance of `RomCounter`.
    ///
    /// # Returns
    /// A new `RomCounter` instance.
    pub fn new(
        computed: bool,
        bios_inst_count: Arc<Vec<AtomicU32>>,
        prog_inst_count: Arc<Vec<AtomicU32>>,
    ) -> Self {
        let rom_counter = RomCounter::new(bios_inst_count, prog_inst_count);
        Self { already_computed: computed, rom_counter }
    }
}

impl BusDevice<u64> for RomCollector {
    /// Processes data received on the bus, updating ROM metrics.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    ///
    /// # Returns
    /// An optional vector of tuples where:
    /// - The first element is the bus ID.
    /// - The second element is always empty indicating there are no derived inputs.
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        debug_assert!(*bus_id == ROM_BUS_ID);

        if !self.already_computed {
            self.rom_counter.measure(data);
        }
    }

    /// Returns the bus IDs associated with this counter.
    ///
    /// # Returns
    /// A vector containing the connected bus ID.
    fn bus_id(&self) -> Vec<BusId> {
        vec![ROM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
