//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use crate::DummyCounter;
use data_bus::DataBusTrait;
use mem_common::MemCounters;
use precomp_arith_eq::ArithEqCounterInputGen;
use precomp_arith_eq_384::ArithEq384CounterInputGen;
use precomp_big_int::Add256CounterInputGen;
use precomp_keccakf::KeccakfCounterInputGen;
use precomp_sha256f::Sha256fCounterInputGen;
use sm_arith::ArithCounterInputGen;
use sm_binary::BinaryCounter;
use sm_main::MainCounter;
use zisk_common::{BusDevice, BusDeviceMetrics, BusId, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID};
use zisk_core::{
    ARITH_EQ_384_OP_TYPE_ID, ARITH_EQ_OP_TYPE_ID, ARITH_OP_TYPE_ID, BIG_INT_OP_TYPE_ID,
    BINARY_E_OP_TYPE_ID, BINARY_OP_TYPE_ID, KECCAK_OP_TYPE_ID, PUB_OUT_OP_TYPE_ID,
    SHA256_OP_TYPE_ID,
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
pub struct StaticDataBus<D> {
    /// Flag indicating whether the bus should only process operation bus related data.
    process_only_operation_bus: bool,

    /// List of devices connected to the bus.
    pub main_counter: MainCounter,
    pub mem_counter: (usize, Option<MemCounters>),
    pub binary_counter: (usize, BinaryCounter),
    pub arith_counter: (usize, ArithCounterInputGen),
    pub keccakf_counter: (usize, KeccakfCounterInputGen),
    pub sha256f_counter: (usize, Sha256fCounterInputGen),
    pub arith_eq_counter: (usize, ArithEqCounterInputGen),
    pub arith_eq_384_counter: (usize, ArithEq384CounterInputGen),
    pub add_256_counter: (usize, Add256CounterInputGen),
    pub rom_counter_id: Option<usize>,
    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>)>,
}

impl StaticDataBus<PayloadType> {
    /// Creates a new `DataBus` instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        process_only_operation_bus: bool,
        mem_counter: (usize, Option<MemCounters>),
        binary_counter: (usize, BinaryCounter),
        arith_counter: (usize, ArithCounterInputGen),
        keccakf_counter: (usize, KeccakfCounterInputGen),
        sha256f_counter: (usize, Sha256fCounterInputGen),
        arith_eq_counter: (usize, ArithEqCounterInputGen),
        arith_eq_384_counter: (usize, ArithEq384CounterInputGen),
        add_256_counter: (usize, Add256CounterInputGen),
        rom_counter_id: Option<usize>,
    ) -> Self {
        Self {
            process_only_operation_bus,
            main_counter: MainCounter::new(),
            mem_counter,
            binary_counter,
            arith_counter,
            keccakf_counter,
            sha256f_counter,
            arith_eq_counter,
            arith_eq_384_counter,
            add_256_counter,
            rom_counter_id,
            pending_transfers: VecDeque::new(),
        }
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
    fn route_data(&mut self, bus_id: BusId, payload: &[PayloadType]) -> bool {
        match bus_id {
            MEM_BUS_ID => {
                let mut _continue = true;
                if !self.process_only_operation_bus {
                    if let Some(mem_counter) = self.mem_counter.1.as_mut() {
                        // If we are not processing only operation bus, we process memory bus data.
                        _continue &= mem_counter.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }
                }
                _continue
            }
            OPERATION_BUS_ID => match payload[1] as u32 {
                PUB_OUT_OP_TYPE_ID => self.main_counter.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                BINARY_OP_TYPE_ID | BINARY_E_OP_TYPE_ID => self.binary_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                ARITH_OP_TYPE_ID => self.arith_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                KECCAK_OP_TYPE_ID => self.keccakf_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                SHA256_OP_TYPE_ID => self.sha256f_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                ARITH_EQ_OP_TYPE_ID => self.arith_eq_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                ARITH_EQ_384_OP_TYPE_ID => self.arith_eq_384_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                BIG_INT_OP_TYPE_ID => self.add_256_counter.1.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
                    None,
                ),
                _ => true,
            },
            _ => true,
        }
    }
}

impl DataBusTrait<PayloadType, Box<dyn BusDeviceMetrics>> for StaticDataBus<PayloadType> {
    #[inline(always)]
    fn write_to_bus(&mut self, bus_id: BusId, payload: &[PayloadType]) -> bool {
        let mut _continue = self.route_data(bus_id, payload);

        while let Some((bus_id, payload)) = self.pending_transfers.pop_front() {
            _continue &= self.route_data(bus_id, &payload);
        }

        _continue
    }

    fn on_close(&mut self) {
        self.main_counter.on_close();
        if let Some(mem_counter) = self.mem_counter.1.as_mut() {
            mem_counter.on_close();
        }
        self.binary_counter.1.on_close();
        self.arith_counter.1.on_close();
        self.keccakf_counter.1.on_close();
        self.sha256f_counter.1.on_close();
        self.arith_eq_counter.1.on_close();
        self.arith_eq_384_counter.1.on_close();
        self.add_256_counter.1.on_close();
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
            (None, Some(Box::new(self.main_counter))),
            (self.rom_counter_id, Some(Box::new(DummyCounter {}))),
            (Some(self.binary_counter.0), Some(Box::new(self.binary_counter.1))),
            (Some(self.arith_counter.0), Some(Box::new(self.arith_counter.1))),
            (Some(self.keccakf_counter.0), Some(Box::new(self.keccakf_counter.1))),
            (Some(self.sha256f_counter.0), Some(Box::new(self.sha256f_counter.1))),
            (Some(self.arith_eq_counter.0), Some(Box::new(self.arith_eq_counter.1))),
            (Some(self.arith_eq_384_counter.0), Some(Box::new(self.arith_eq_384_counter.1))),
            (Some(self.add_256_counter.0), Some(Box::new(self.add_256_counter.1))),
        ];

        if let Some(mem_counter) = self.mem_counter.1 {
            counters.insert(1, (Some(self.mem_counter.0), Some(Box::new(mem_counter))));
        }

        counters
    }
}
