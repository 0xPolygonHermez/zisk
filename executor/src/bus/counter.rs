//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use crate::{pub_outs_collector::PubOutsCollector, BuiltinCounters, PrecompileCounters};
use data_bus::DataBusTrait;
use fields::PrimeField64;
use mem_common::MemCounters;
use precomp_dma::DmaCounterInputGen;
use precompiles_common::MemCounterProcessor;
use sm_arith::ArithCounterInputGen;
use sm_binary::BinaryCounter;
use zisk_common::{BusDeviceMetrics, BusId, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{
    ARITH_OP_TYPE_ID, BINARY_E_OP_TYPE_ID, BINARY_OP_TYPE_ID, DMA_OP_TYPE_ID, PUB_OUT_OP_TYPE_ID,
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
    pub_outs_collector: PubOutsCollector,

    /// Memory-related counter.
    mem_counter: (usize, Option<MemCounters>),

    /// Arithmetic operation counter.
    arith_counter: (usize, ArithCounterInputGen),

    /// Binary operation counter.
    binary_counter: (usize, BinaryCounter),

    /// DMA operation counter.
    dma_counter: (usize, DmaCounterInputGen),

    /// Precompile operation counters.
    precompiles: PrecompileCounters<F>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>, Vec<D>)>,
}

impl<F: PrimeField64> StaticDataBus<PayloadType, F> {
    /// Constructs a counter-phase data bus via static dispatch — no
    /// `StaticSMBundle` required. Callable on the standalone path.
    pub fn build(is_asm_emulator: bool) -> Self {
        let builtins = BuiltinCounters::build::<F>(is_asm_emulator);
        let precompiles = PrecompileCounters::<F>::build(is_asm_emulator);

        Self {
            process_only_operation_bus: is_asm_emulator,
            pub_outs_collector: PubOutsCollector::new(),
            mem_counter: builtins.mem,
            arith_counter: builtins.arith,
            binary_counter: builtins.binary,
            dma_counter: builtins.dma,
            precompiles,
            pending_transfers: VecDeque::new(),
        }
    }

    /// Drains the accumulated public outputs from the embedded `PubOutsCollector`,
    /// leaving the collector with an empty Vec. Must be called BEFORE
    /// `into_devices`, which consumes the bus.
    #[inline]
    pub fn take_pub_outs(&mut self) -> PubOutsCollector {
        std::mem::take(&mut self.pub_outs_collector)
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
                DMA_OP_TYPE_ID => self.dma_counter.1.process_data(
                    &bus_id,
                    data,
                    data_ext,
                    &mut MemCounterProcessor::new(self.mem_counter.1.as_mut()),
                ),
                op => self.precompiles.dispatch_op(op, &bus_id, data, self.mem_counter.1.as_mut()),
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

    fn into_devices(mut self, execute_on_close: bool) -> Vec<(usize, Box<dyn BusDeviceMetrics>)> {
        if execute_on_close {
            self.on_close();
        }

        let mut counters: Vec<(usize, Box<dyn BusDeviceMetrics>)> = vec![
            (self.binary_counter.0, Box::new(self.binary_counter.1)),
            (self.arith_counter.0, Box::new(self.arith_counter.1)),
        ];
        counters.extend(self.precompiles.into_device_entries());
        counters.push((self.dma_counter.0, Box::new(self.dma_counter.1)));

        if let Some(mem_counter) = self.mem_counter.1 {
            counters.insert(0, (self.mem_counter.0, Box::new(mem_counter)));
        }

        counters
    }
}
