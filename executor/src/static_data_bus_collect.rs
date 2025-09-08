//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use data_bus::DataBusTrait;
use precomp_arith_eq::ArithEqCollector;
use precomp_arith_eq::ArithEqCounterInputGen;
use precomp_keccakf::KeccakfCollector;
use precomp_keccakf::KeccakfCounterInputGen;
use precomp_sha256f::Sha256fCollector;
use precomp_sha256f::Sha256fCounterInputGen;
use sm_arith::ArithCounterInputGen;
use sm_arith::ArithInstanceCollector;
use sm_binary::{BinaryAddCollector, BinaryBasicCollector, BinaryExtensionCollector};
use sm_mem::{MemAlignCollector, MemHelpers, MemModuleCollector};
use sm_rom::RomCollector;
use zisk_common::{
    BusDevice, BusId, MemBusData, PayloadType, MEM_BUS_ID, OP, OPERATION_BUS_ID, OP_TYPE,
    ROM_BUS_ID,
};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

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

    /// ROM collector
    pub rom_collector: Vec<(usize, RomCollector)>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>)>,
}

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
        rom_collector: Vec<(usize, RomCollector)>,
        arith_eq_inputs_generator: ArithEqCounterInputGen,
        keccakf_inputs_generator: KeccakfCounterInputGen,
        sha256f_inputs_generator: Sha256fCounterInputGen,
        arith_inputs_generator: ArithCounterInputGen,
    ) -> Self {
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
            rom_collector,
            arith_eq_inputs_generator,
            keccakf_inputs_generator,
            sha256f_inputs_generator,
            arith_inputs_generator,
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
    fn route_data(&mut self, bus_id: BusId, payload: &[PayloadType]) {
        match bus_id {
            MEM_BUS_ID => {
                // Pre-compute values once and reuse
                let addr = MemBusData::get_addr(payload);
                let bytes = MemBusData::get_bytes(payload);
                let is_unaligned = !MemHelpers::is_aligned(addr, bytes);
                let unaligned_double = is_unaligned && MemHelpers::is_double(addr, bytes);

                // Process mem collectors - inverted condition to avoid continue
                for (_, mem_collector) in &mut self.mem_collector {
                    if !mem_collector.skip_collector(addr, unaligned_double) {
                        mem_collector.process_data(&bus_id, payload, &mut self.pending_transfers);
                    }
                }

                // Only process align collectors if needed
                if is_unaligned {
                    for (_, mem_align_collector) in &mut self.mem_align_collector {
                        mem_align_collector.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                        );
                    }
                }
            }
            OPERATION_BUS_ID => {
                let op_type = payload[OP_TYPE] as u32;
                match op_type {
                    op if op == ZiskOperationType::Binary as u32 => {
                        if payload[OP] as u8 == ZiskOp::Add.code() {
                            for (_, binary_add_collector) in &mut self.binary_add_collector {
                                binary_add_collector.process_data(
                                    &bus_id,
                                    payload,
                                    &mut self.pending_transfers,
                                );
                            }
                        } else {
                            for (_, binary_basic_collector) in &mut self.binary_basic_collector {
                                binary_basic_collector.process_data(
                                    &bus_id,
                                    payload,
                                    &mut self.pending_transfers,
                                );
                            }
                        }
                    }
                    op if op == ZiskOperationType::BinaryE as u32 => {
                        for (_, binary_extension_collector) in &mut self.binary_extension_collector
                        {
                            binary_extension_collector.process_data(
                                &bus_id,
                                payload,
                                &mut self.pending_transfers,
                            );
                        }
                    }
                    op if op == ZiskOperationType::Arith as u32 => {
                        for (_, arith_collector) in &mut self.arith_collector {
                            arith_collector.process_data(
                                &bus_id,
                                payload,
                                &mut self.pending_transfers,
                            );
                        }

                        self.arith_inputs_generator.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                        );
                    }
                    op if op == ZiskOperationType::Keccak as u32 => {
                        for (_, keccakf_collector) in &mut self.keccakf_collector {
                            keccakf_collector.process_data(
                                &bus_id,
                                payload,
                                &mut self.pending_transfers,
                            );
                        }
                        self.keccakf_inputs_generator.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                        );
                    }
                    op if op == ZiskOperationType::Sha256 as u32 => {
                        for (_, sha256f_collector) in &mut self.sha256f_collector {
                            sha256f_collector.process_data(
                                &bus_id,
                                payload,
                                &mut self.pending_transfers,
                            );
                        }
                        self.sha256f_inputs_generator.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                        );
                    }
                    op if op == ZiskOperationType::ArithEq as u32 => {
                        for (_, arith_eq_collector) in &mut self.arith_eq_collector {
                            arith_eq_collector.process_data(
                                &bus_id,
                                payload,
                                &mut self.pending_transfers,
                            );
                        }

                        self.arith_eq_inputs_generator.process_data(
                            &bus_id,
                            payload,
                            &mut self.pending_transfers,
                        );
                    }
                    _ => {}
                }
            }
            ROM_BUS_ID => {
                for (_, rom_collector) in &mut self.rom_collector {
                    rom_collector.process_data(&bus_id, payload, &mut self.pending_transfers);
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

        for (id, collector) in self.rom_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        // Add generators
        result.push((
            None,
            Some(Box::new(self.arith_eq_inputs_generator) as Box<dyn BusDevice<PayloadType>>),
        ));
        result.push((
            None,
            Some(Box::new(self.keccakf_inputs_generator) as Box<dyn BusDevice<PayloadType>>),
        ));
        result.push((
            None,
            Some(Box::new(self.sha256f_inputs_generator) as Box<dyn BusDevice<PayloadType>>),
        ));
        result.push((
            None,
            Some(Box::new(self.arith_inputs_generator) as Box<dyn BusDevice<PayloadType>>),
        ));

        result
    }
}
