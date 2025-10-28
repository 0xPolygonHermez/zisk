//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use data_bus::DataBusTrait;
use precomp_arith_eq::ArithEqCollector;
use precomp_arith_eq::ArithEqCounterInputGen;
use precomp_arith_eq_384::ArithEq384Collector;
use precomp_arith_eq_384::ArithEq384CounterInputGen;
use precomp_big_int::Add256Collector;
use precomp_big_int::Add256CounterInputGen;
use precomp_keccakf::KeccakfCollector;
use precomp_keccakf::KeccakfCounterInputGen;
use precomp_sha256f::Sha256fCollector;
use precomp_sha256f::Sha256fCounterInputGen;
use sm_arith::ArithCounterInputGen;
use sm_arith::ArithInstanceCollector;
use sm_binary::{BinaryAddCollector, BinaryBasicCollector, BinaryExtensionCollector};
use sm_mem::{MemAlignCollector, MemModuleCollector};
use sm_rom::RomCollector;
use zisk_common::{
    BusDevice, BusId, MemCollectorInfo, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID, OP_TYPE,
    ROM_BUS_ID,
};
use zisk_core::ZiskOperationType;

/// A bus system facilitating communication between multiple publishers and subscribers.
///
/// The `DataBus` allows devices to register for specific bus IDs or act as global (omni) devices.
/// It routes payloads to registered devices and handles data transfers efficiently.
///
/// # Type Parameters
/// * `D` - The type of data payloads handled by the bus.
/// * `BD` - The type of devices (subscribers) connected to the bus, implementing the `BusDevice`
///   trait.
pub struct StaticDataBusCollect<D> {
    /// Memory-related collectors (grouped for cache locality)
    pub mem_collector: Vec<(usize, MemModuleCollector)>,
    pub mem_align_collector: Vec<(usize, MemAlignCollector)>,

    /// Binary operation collectors (grouped for cache locality)
    pub binary_basic_collector: Vec<(usize, BinaryBasicCollector)>,
    pub binary_add_collector: Vec<(usize, BinaryAddCollector)>,
    pub binary_extension_collector: Vec<(usize, BinaryExtensionCollector)>,

    /// Arithmetic collectors (grouped for cache locality)
    pub arith_collector: Vec<(usize, ArithInstanceCollector)>,
    pub arith_inputs_generator: ArithCounterInputGen,

    /// Cryptographic hash collectors (grouped for cache locality)
    pub keccakf_collector: Vec<(usize, KeccakfCollector)>,
    pub keccakf_inputs_generator: KeccakfCounterInputGen,
    pub sha256f_collector: Vec<(usize, Sha256fCollector)>,
    pub sha256f_inputs_generator: Sha256fCounterInputGen,

    /// Arithmetic equality collectors
    pub arith_eq_collector: Vec<(usize, ArithEqCollector)>,
    pub arith_eq_inputs_generator: ArithEqCounterInputGen,

    /// ArithEq384 collectors
    pub arith_eq_384_collector: Vec<(usize, ArithEq384Collector)>,
    pub arith_eq_384_inputs_generator: ArithEq384CounterInputGen,

    /// Add256 collectors
    pub add256_collector: Vec<(usize, Add256Collector)>,
    pub add256_inputs_generator: Add256CounterInputGen,

    /// ROM collector
    pub rom_collector: Vec<(usize, RomCollector)>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>)>,

    mem_collectors_info: Vec<MemCollectorInfo>,
}

const BINARY_TYPE: u64 = ZiskOperationType::Binary as u64;
const BINARY_E_TYPE: u64 = ZiskOperationType::BinaryE as u64;
const ARITH_TYPE: u64 = ZiskOperationType::Arith as u64;
const KECCAK_TYPE: u64 = ZiskOperationType::Keccak as u64;
const SHA256_TYPE: u64 = ZiskOperationType::Sha256 as u64;
const ARITH_EQ_TYPE: u64 = ZiskOperationType::ArithEq as u64;
const ARITH_EQ_384_TYPE: u64 = ZiskOperationType::ArithEq384 as u64;
const BIG_INT_OP_TYPE_ID: u64 = ZiskOperationType::BigInt as u64;

impl StaticDataBusCollect<PayloadType> {
    /// Creates a new `DataBus` instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mem_collector: Vec<(usize, MemModuleCollector)>,
        mem_align_collector: Vec<(usize, MemAlignCollector)>,
        binary_basic_collector: Vec<(usize, BinaryBasicCollector)>,
        binary_add_collector: Vec<(usize, BinaryAddCollector)>,
        binary_extension_collector: Vec<(usize, BinaryExtensionCollector)>,
        arith_collector: Vec<(usize, ArithInstanceCollector)>,
        keccakf_collector: Vec<(usize, KeccakfCollector)>,
        sha256f_collector: Vec<(usize, Sha256fCollector)>,
        arith_eq_collector: Vec<(usize, ArithEqCollector)>,
        arith_eq_384_collector: Vec<(usize, ArithEq384Collector)>,
        add256_collector: Vec<(usize, Add256Collector)>,
        rom_collector: Vec<(usize, RomCollector)>,
        arith_eq_inputs_generator: ArithEqCounterInputGen,
        arith_eq_384_inputs_generator: ArithEq384CounterInputGen,
        keccakf_inputs_generator: KeccakfCounterInputGen,
        sha256f_inputs_generator: Sha256fCounterInputGen,
        arith_inputs_generator: ArithCounterInputGen,
        add256_inputs_generator: Add256CounterInputGen,
    ) -> Self {
        let mem_collectors_info: Vec<MemCollectorInfo> =
            mem_collector.iter().map(|(_, collector)| collector.get_mem_collector_info()).collect();

        Self {
            mem_collector,
            mem_align_collector,
            binary_basic_collector,
            binary_add_collector,
            binary_extension_collector,
            arith_collector,
            keccakf_collector,
            sha256f_collector,
            arith_eq_collector,
            arith_eq_384_collector,
            add256_collector,
            rom_collector,
            arith_eq_inputs_generator,
            arith_eq_384_inputs_generator,
            keccakf_inputs_generator,
            sha256f_inputs_generator,
            arith_inputs_generator,
            add256_inputs_generator,
            pending_transfers: VecDeque::with_capacity(64),
            mem_collectors_info,
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
    fn route_data(&mut self, bus_id: BusId, payload: &[PayloadType]) {
        match bus_id {
            MEM_BUS_ID => {
                // Process mem collectors - inverted condition to avoid continue
                for (_, mem_collector) in &mut self.mem_collector {
                    mem_collector.process_data(&bus_id, payload, &mut self.pending_transfers, None);
                }

                // Only process align collectors if needed
                for (_, mem_align_collector) in &mut self.mem_align_collector {
                    mem_align_collector.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        None,
                    );
                }
            }
            OPERATION_BUS_ID => match payload[OP_TYPE] {
                BINARY_TYPE => {
                    for (_, binary_add_collector) in &mut self.binary_add_collector {
                        binary_add_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    for (_, binary_basic_collector) in &mut self.binary_basic_collector {
                        binary_basic_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }
                }
                BINARY_E_TYPE => {
                    for (_, binary_extension_collector) in &mut self.binary_extension_collector {
                        binary_extension_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }
                }
                ARITH_TYPE => {
                    for (_, arith_collector) in &mut self.arith_collector {
                        arith_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    self.arith_inputs_generator.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        None,
                    );
                }
                KECCAK_TYPE => {
                    for (_, keccakf_collector) in &mut self.keccakf_collector {
                        keccakf_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    self.keccakf_inputs_generator.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        Some(&self.mem_collectors_info),
                    );
                }
                SHA256_TYPE => {
                    for (_, sha256f_collector) in &mut self.sha256f_collector {
                        sha256f_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    self.sha256f_inputs_generator.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        Some(&self.mem_collectors_info),
                    );
                }
                ARITH_EQ_TYPE => {
                    for (_, arith_eq_collector) in &mut self.arith_eq_collector {
                        arith_eq_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    self.arith_eq_inputs_generator.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        Some(&self.mem_collectors_info),
                    );
                }
                ARITH_EQ_384_TYPE => {
                    for (_, arith_eq_384_collector) in &mut self.arith_eq_384_collector {
                        arith_eq_384_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    self.arith_eq_384_inputs_generator.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        Some(&self.mem_collectors_info),
                    );
                }
                BIG_INT_OP_TYPE_ID => {
                    for (_, add256_collector) in &mut self.add256_collector {
                        add256_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                            None,
                        );
                    }

                    self.add256_inputs_generator.process_data(
                        &bus_id,
                        payload,
                        &mut self.pending_transfers,
                        Some(&self.mem_collectors_info),
                    );
                }
                _ => {}
            },
            ROM_BUS_ID => {
                for (_, rom_collector) in &mut self.rom_collector {
                    rom_collector.process_data(&bus_id, payload, &mut self.pending_transfers, None);
                }
            }
            _ => {}
        }
    }
}

impl DataBusTrait<PayloadType, Box<dyn BusDevice<PayloadType>>>
    for StaticDataBusCollect<PayloadType>
{
    #[inline(always)]
    fn write_to_bus(&mut self, bus_id: BusId, payload: &[PayloadType]) -> bool {
        self.route_data(bus_id, payload);

        // Process all pending transfers in a batch to improve cache locality
        while let Some((pending_bus_id, pending_payload)) = self.pending_transfers.pop_front() {
            self.route_data(pending_bus_id, &pending_payload);
        }

        true
    }

    fn on_close(&mut self) {}

    fn into_devices(
        mut self,
        execute_on_close: bool,
    ) -> Vec<(Option<usize>, Option<Box<dyn BusDevice<PayloadType>>>)> {
        if execute_on_close {
            self.on_close();
        }

        let mut result = Vec::new();

        // Add all collectors to the result
        for (id, collector) in self.mem_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.mem_align_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.binary_basic_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.binary_add_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.binary_extension_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.arith_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.keccakf_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.sha256f_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.arith_eq_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.arith_eq_384_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.add256_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.rom_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        result
    }
}
