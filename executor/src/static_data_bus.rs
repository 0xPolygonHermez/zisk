//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use crate::{BuiltinCounters, DummyCounter, PrecompileCounters, StaticSMBundle};
use anyhow::Result;
use data_bus::DataBusTrait;
use fields::PrimeField64;
use mem_common::MemCounters;
use precomp_dma::DmaCounterInputGen;
use precompiles_common::MemCounterProcessor;
use sm_arith::ArithCounterInputGen;
use sm_binary::BinaryCounter;
use sm_main::PubOutsCollector;
use zisk_common::{BusDeviceMetrics, BusId, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{
    ARITH_EQ_384_OP_TYPE_ID, ARITH_EQ_OP_TYPE_ID, ARITH_OP_TYPE_ID, BIG_INT_OP_TYPE_ID,
    BINARY_E_OP_TYPE_ID, BINARY_OP_TYPE_ID, BLAKE2_OP_TYPE_ID, DMA_OP_TYPE_ID, KECCAK_OP_TYPE_ID,
    POSEIDON2_OP_TYPE_ID, PUB_OUT_OP_TYPE_ID, SHA256_OP_TYPE_ID,
};

/// A bus system facilitating communication between multiple publishers and subscribers.
///
/// The `DataBus` allows devices to register for specific bus IDs or act as global (omni) devices.
/// It routes payloads to registered devices and handles data transfers efficiently.
///
/// # Type Parameters
/// * `D` - The type of data payloads handled by the bus.
/// * `BD` - The type of devices (subscribers) connected to the bus, implementing the `BusDevice`
///   trait.
pub struct StaticDataBus<D, F: PrimeField64> {
    /// Flag indicating whether the bus should only process operation bus related data.
    process_only_operation_bus: bool,

    /// List of devices connected to the bus.
    pub pub_outs_collector: PubOutsCollector,
    pub mem_counter: (usize, Option<MemCounters>),
    pub binary_counter: (usize, BinaryCounter),
    pub arith_counter: (usize, ArithCounterInputGen),
    pub precompiles: PrecompileCounters<F>,
    pub dma_counter: (usize, DmaCounterInputGen),
    pub rom_counter_id: Option<usize>,
    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>, Vec<D>)>,
}

impl<F: PrimeField64> StaticDataBus<PayloadType, F> {
    /// Constructs a counter-phase data bus from the executor's bundle.
    /// Iterates the bundle's entries once via `BuiltinCounters` and
    /// `PrecompileCounters`, then wires their slots into `new`.
    /// Mirrors the `from_bundle` constructors on the wrapper types.
    pub fn from_bundle(bundle: &StaticSMBundle<F>, is_asm_emulator: bool) -> Result<Self> {
        let builtins = BuiltinCounters::from_bundle(bundle, is_asm_emulator)?;
        let precompiles = PrecompileCounters::from_bundle(bundle, is_asm_emulator)?;

        Ok(Self::new(
            is_asm_emulator,
            builtins.mem,
            builtins.binary,
            builtins.arith,
            precompiles,
            builtins.dma,
            Some(0),
        ))
    }

    /// Creates a new `DataBus` instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        process_only_operation_bus: bool,
        mem_counter: (usize, Option<MemCounters>),
        binary_counter: (usize, BinaryCounter),
        arith_counter: (usize, ArithCounterInputGen),
        precompiles: PrecompileCounters<F>,
        dma_counter: (usize, DmaCounterInputGen),
        rom_counter_id: Option<usize>,
    ) -> Self {
        Self {
            process_only_operation_bus,
            pub_outs_collector: PubOutsCollector::new(),
            mem_counter,
            binary_counter,
            arith_counter,
            precompiles,
            dma_counter,
            rom_counter_id,
            pending_transfers: VecDeque::new(),
        }
    }

    /// Drains the accumulated public outputs from the embedded `PubOutsCollector`,
    /// leaving the collector with an empty Vec. Must be called BEFORE
    /// `into_devices`, which consumes the bus.
    #[inline]
    pub fn take_pub_outs(&mut self) -> Vec<(u64, u32)> {
        std::mem::take(&mut self.pub_outs_collector.0)
    }

    /// Routes data to the devices subscribed to a specific bus ID or global devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to route the data to.
    /// * `payload` - A reference to the data payload being routed.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn route_data(
        &mut self,
        bus_id: BusId,
        data: &[PayloadType],
        data_ext: &[PayloadType],
    ) -> bool {
        match bus_id {
            MEM_BUS_ID => {
                let mut _continue = true;
                if !self.process_only_operation_bus {
                    if let Some(mem_counter) = self.mem_counter.1.as_mut() {
                        // If we are not processing only operation bus, we process memory bus data.
                        _continue &= mem_counter.process_data(&bus_id, data);
                    }
                }
                _continue
            }
            OPERATION_BUS_ID => match data[OP_TYPE] as u32 {
                PUB_OUT_OP_TYPE_ID => {
                    self.pub_outs_collector.process_data(data);
                    true
                }
                BINARY_OP_TYPE_ID | BINARY_E_OP_TYPE_ID => {
                    self.binary_counter.1.process_data(&bus_id, data)
                }
                ARITH_OP_TYPE_ID => {
                    self.arith_counter.1.process_data(&bus_id, data, &mut self.pending_transfers)
                }
                KECCAK_OP_TYPE_ID => self.precompiles.keccakf.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                SHA256_OP_TYPE_ID => self.precompiles.sha256f.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                POSEIDON2_OP_TYPE_ID => self.precompiles.poseidon2.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                BLAKE2_OP_TYPE_ID => self.precompiles.blake2.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                ARITH_EQ_OP_TYPE_ID => self.precompiles.arith_eq.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                ARITH_EQ_384_OP_TYPE_ID => self.precompiles.arith_eq384.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                BIG_INT_OP_TYPE_ID => self.precompiles.add256.1.process_data(
                    &bus_id,
                    data,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                DMA_OP_TYPE_ID => self.dma_counter.1.process_data(
                    &bus_id,
                    data,
                    data_ext,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                _ => true,
            },
            _ => true,
        }
    }
}

impl<F: PrimeField64> DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>>
    for StaticDataBus<PayloadType, F>
{
    #[inline(always)]
    fn write_to_bus(
        &mut self,
        bus_id: BusId,
        data: &[PayloadType],
        data_ext: &[PayloadType],
    ) -> bool {
        let mut _continue = self.route_data(bus_id, data, data_ext);

        while let Some((bus_id, data, data_ext)) = self.pending_transfers.pop_front() {
            _continue &= self.route_data(bus_id, &data, &data_ext);
        }

        _continue
    }

    fn on_close(&mut self) {
        if let Some(mem_counter) = self.mem_counter.1.as_mut() {
            mem_counter.close();
        }
    }

    fn into_devices(
        mut self,
        execute_on_close: bool,
    ) -> Vec<(Option<usize>, Option<Box<dyn BusDeviceMetrics>>)> {
        if execute_on_close {
            self.on_close();
        }

        #[allow(clippy::type_complexity)]
        let mut counters: Vec<(Option<usize>, Option<Box<dyn BusDeviceMetrics>>)> = vec![
            (self.rom_counter_id, Some(Box::new(DummyCounter {}))),
            (Some(self.binary_counter.0), Some(Box::new(self.binary_counter.1))),
            (Some(self.arith_counter.0), Some(Box::new(self.arith_counter.1))),
        ];
        counters.extend(self.precompiles.into_device_entries());
        counters.push((Some(self.dma_counter.0), Some(Box::new(self.dma_counter.1))));

        if let Some(mem_counter) = self.mem_counter.1 {
            counters.insert(1, (Some(self.mem_counter.0), Some(Box::new(mem_counter))));
        }

        counters
    }
}
