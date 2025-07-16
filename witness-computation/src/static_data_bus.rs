//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use data_bus::DataBusTrait;
use precomp_arith_eq::ArithEqCounterInputGen;
use precomp_keccakf::KeccakfCounterInputGen;
use precomp_sha256f::Sha256fCounterInputGen;
use sm_arith::ArithCounterInputGen;
use sm_binary::BinaryCounter;
use sm_main::MainCounter;
use sm_mem::MemCounters;
use zisk_common::{BusDevice, BusDeviceMetrics, BusId, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID};
use zisk_core::{
    ARITH_EQ_OP_TYPE_ID, ARITH_OP_TYPE_ID, BINARY_E_OP_TYPE_ID, BINARY_OP_TYPE_ID,
    KECCAK_OP_TYPE_ID, PUB_OUT_OP_TYPE_ID, SHA256_OP_TYPE_ID,
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
    pub mem_counter: MemCounters,
    pub binary_counter: BinaryCounter,
    pub arith_counter: ArithCounterInputGen,
    pub keccakf_counter: KeccakfCounterInputGen,
    pub sha256f_counter: Sha256fCounterInputGen,
    pub arith_eq_counter: ArithEqCounterInputGen,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>)>,
}

impl StaticDataBus<PayloadType> {
    /// Creates a new `DataBus` instance.
    pub fn new(
        process_only_operation_bus: bool,
        mem_counter: MemCounters,
        binary_counter: BinaryCounter,
        arith_counter: ArithCounterInputGen,
        keccakf_counter: KeccakfCounterInputGen,
        sha256f_counter: Sha256fCounterInputGen,
        arith_eq_counter: ArithEqCounterInputGen,
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
                    // If we are not processing only operation bus, we process memory bus data.
                    _continue &= self.mem_counter.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                    );
                }

                _continue
            }
            OPERATION_BUS_ID => match payload[1] as u32 {
                PUB_OUT_OP_TYPE_ID => {
                    self.main_counter.process_data(&bus_id, payload, &mut self.pending_transfers)
                }
                BINARY_OP_TYPE_ID | BINARY_E_OP_TYPE_ID => {
                    self.binary_counter.process_data(&bus_id, payload, &mut self.pending_transfers)
                }
                ARITH_OP_TYPE_ID => {
                    self.arith_counter.process_data(&bus_id, payload, &mut self.pending_transfers)
                }
                KECCAK_OP_TYPE_ID => {
                    self.keccakf_counter.process_data(&bus_id, payload, &mut self.pending_transfers)
                }
                SHA256_OP_TYPE_ID => {
                    self.sha256f_counter.process_data(&bus_id, payload, &mut self.pending_transfers)
                }
                ARITH_EQ_OP_TYPE_ID => self.arith_eq_counter.process_data(
                    &bus_id,
                    payload,
                    &mut self.pending_transfers,
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
        self.mem_counter.on_close();
        self.binary_counter.on_close();
        self.arith_counter.on_close();
        self.keccakf_counter.on_close();
        self.sha256f_counter.on_close();
        self.arith_eq_counter.on_close();
    }

    fn into_devices(
        mut self,
        execute_on_close: bool,
    ) -> Vec<(Option<usize>, Option<Box<dyn BusDeviceMetrics>>)> {
        if execute_on_close {
            self.on_close();
        }

        let StaticDataBus {
            process_only_operation_bus: _,
            main_counter,
            mem_counter,
            binary_counter,
            arith_counter,
            keccakf_counter,
            sha256f_counter,
            arith_eq_counter,
            pending_transfers: _,
        } = self;

        #[allow(clippy::type_complexity)]
        let counters: Vec<(Option<usize>, Option<Box<dyn BusDeviceMetrics>>)> = vec![
            (None, Some(Box::new(main_counter))),
            (None, Some(Box::new(mem_counter))),
            (None, None),
            (None, Some(Box::new(binary_counter))),
            (None, Some(Box::new(arith_counter))),
            (None, Some(Box::new(keccakf_counter))),
            (None, Some(Box::new(sha256f_counter))),
            (None, Some(Box::new(arith_eq_counter))),
        ];

        counters
    }
}
