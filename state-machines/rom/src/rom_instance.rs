//! The `RomInstance` performs the witness computation based on the provided ROM execution plan
//!
//! It is responsible for computing witnesses for ROM-related execution plans,

use std::sync::{atomic::AtomicU32, Arc};

use crate::{rom_counter::RomCounter, RomSM};
use data_bus::{BusDevice, BusId, PayloadType, ROM_BUS_ID};
use p3_field::PrimeField;
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use sm_common::{
    BusDeviceWrapper, CheckPoint, CounterStats, Instance, InstanceCtx, InstanceType, Metrics,
};
use zisk_common::ChunkId;
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
    bios_inst_count: Arc<Vec<AtomicU32>>,

    /// Shared program instruction counter for monitoring ROM operations.
    prog_inst_count: Arc<Vec<AtomicU32>>,
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
    ) -> Self {
        Self { zisk_rom, ictx, bios_inst_count, prog_inst_count }
    }
}

impl<F: PrimeField> Instance<F> for RomInstance {
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
        &mut self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<BusDeviceWrapper<PayloadType>>)>,
    ) -> Option<AirInstance<F>> {
        let collectors: Vec<_> = collectors
            .into_iter()
            .map(|(_, mut collector)| {
                collector.detach_device().as_any().downcast::<RomCollector>().unwrap()
            })
            .collect();

        let mut counter_stats_total =
            CounterStats::new(self.bios_inst_count.clone(), self.prog_inst_count.clone());

        for collector in collectors {
            counter_stats_total += &collector.rom_counter.counter_stats;
        }

        Some(RomSM::compute_witness(&self.zisk_rom, &counter_stats_total))
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
    fn build_inputs_collector(
        &self,
        _chunk_id: ChunkId,
    ) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(RomCollector::new(
            self.bios_inst_count.clone(),
            self.prog_inst_count.clone(),
        )))
    }
}

pub struct RomCollector {
    /// Execution statistics counter for ROM instructions.
    pub rom_counter: RomCounter,
}

impl RomCollector {
    /// Creates a new instance of `RomCounter`.
    ///
    /// # Returns
    /// A new `RomCounter` instance.
    pub fn new(bios_inst_count: Arc<Vec<AtomicU32>>, prog_inst_count: Arc<Vec<AtomicU32>>) -> Self {
        let rom_counter = RomCounter::new(bios_inst_count, prog_inst_count);
        Self { rom_counter }
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
    #[inline]
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == ROM_BUS_ID);

        self.rom_counter.measure(data);

        None
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
