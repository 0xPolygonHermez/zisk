//! The `DataBus` module facilitates communication between publishers and subscribers using a bus
//! system. Subscribers, referred to as `BusDevice`, can listen to specific bus IDs or act as
//! omnipresent devices that process all data sent to the bus. This module provides mechanisms to
//! send data, route it to the appropriate subscribers, and manage device connections.
use std::collections::VecDeque;

use data_bus::DataBusTrait;
use fields::PrimeField64;
use precomp_arith_eq::ArithEqCollector;
use precomp_arith_eq::ArithEqCounterInputGen;
use precomp_arith_eq_384::ArithEq384Collector;
use precomp_arith_eq_384::ArithEq384CounterInputGen;
use precomp_big_int::Add256Collector;
use precomp_big_int::Add256CounterInputGen;
use precomp_dma::Dma64AlignedCollector;
use precomp_dma::DmaCollector;
use precomp_dma::DmaCounterInputGen;
use precomp_dma::DmaPrePostCollector;
use precomp_dma::DmaUnalignedCollector;
use precomp_keccakf::KeccakfCollector;
use precomp_keccakf::KeccakfCounterInputGen;
use precomp_poseidon2::Poseidon2Collector;
use precomp_poseidon2::Poseidon2CounterInputGen;
use precomp_sha256f::Sha256fCollector;
use precomp_sha256f::Sha256fCounterInputGen;
use precompiles_common::{MemCollectorProcessor, MemProcessor};
use sm_arith::ArithCounterInputGen;
use sm_arith::ArithInstanceCollector;
use sm_binary::{BinaryAddCollector, BinaryBasicCollector, BinaryExtensionCollector};
use sm_mem::{MemAlignCollector, MemModuleCollector};
use sm_rom::RomCollector;
use zisk_common::{
    BusDevice, BusId, PayloadType, MEM_BUS_ID, OPERATION_BUS_ID, OP_TYPE, ROM_BUS_ID,
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
pub struct StaticDataBusCollect<D, F: PrimeField64> {
    /// Memory-related collectors (grouped for cache locality)
    pub mem_collector: Vec<(usize, MemModuleCollector)>,
    pub mem_align_collector: Vec<(usize, MemAlignCollector)>,

    /// Binary operation collectors (grouped for cache locality)
    pub binary_basic_collector: Vec<(usize, BinaryBasicCollector<F>)>,
    pub binary_add_collector: Vec<(usize, BinaryAddCollector<F>)>,
    pub binary_extension_collector: Vec<(usize, BinaryExtensionCollector<F>)>,

    /// Arithmetic collectors (grouped for cache locality)
    pub arith_collector: Vec<(usize, ArithInstanceCollector<F>)>,
    pub arith_inputs_generator: ArithCounterInputGen,

    /// Cryptographic hash collectors (grouped for cache locality)
    pub keccakf_collector: Vec<(usize, KeccakfCollector)>,
    pub keccakf_inputs_generator: KeccakfCounterInputGen,
    pub sha256f_collector: Vec<(usize, Sha256fCollector)>,
    pub sha256f_inputs_generator: Sha256fCounterInputGen,
    pub poseidon2_collector: Vec<(usize, Poseidon2Collector)>,
    pub poseidon2_inputs_generator: Poseidon2CounterInputGen,

    /// Arithmetic equality collectors
    pub arith_eq_collector: Vec<(usize, ArithEqCollector)>,
    pub arith_eq_inputs_generator: ArithEqCounterInputGen,

    /// ArithEq384 collectors
    pub arith_eq_384_collector: Vec<(usize, ArithEq384Collector)>,
    pub arith_eq_384_inputs_generator: ArithEq384CounterInputGen,

    /// Add256 collectors
    pub add256_collector: Vec<(usize, Add256Collector)>,
    pub add256_inputs_generator: Add256CounterInputGen,

    /// Dma collectors
    pub dma_collector: Vec<(usize, DmaCollector)>,
    pub dma_pre_post_collector: Vec<(usize, DmaPrePostCollector)>,
    pub dma_64_aligned_collector: Vec<(usize, Dma64AlignedCollector)>,
    pub dma_unaligned_collector: Vec<(usize, DmaUnalignedCollector)>,
    pub dma_inputs_generator: DmaCounterInputGen,

    /// ROM collector
    pub rom_collector: Vec<(usize, RomCollector)>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>, Vec<D>)>,
}

const BINARY_TYPE: u64 = ZiskOperationType::Binary as u64;
const BINARY_E_TYPE: u64 = ZiskOperationType::BinaryE as u64;
const ARITH_TYPE: u64 = ZiskOperationType::Arith as u64;
const KECCAK_TYPE: u64 = ZiskOperationType::Keccak as u64;
const SHA256_TYPE: u64 = ZiskOperationType::Sha256 as u64;
const POSEIDON2_TYPE: u64 = ZiskOperationType::Poseidon2 as u64;
const ARITH_EQ_TYPE: u64 = ZiskOperationType::ArithEq as u64;
const ARITH_EQ_384_TYPE: u64 = ZiskOperationType::ArithEq384 as u64;
const BIG_INT_OP_TYPE_ID: u64 = ZiskOperationType::BigInt as u64;
const DMA_OP_TYPE_ID: u64 = ZiskOperationType::Dma as u64;

impl<F: PrimeField64> StaticDataBusCollect<PayloadType, F> {
    /// Creates a new `DataBus` instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mem_collector: Vec<(usize, MemModuleCollector)>,
        mem_align_collector: Vec<(usize, MemAlignCollector)>,
        binary_basic_collector: Vec<(usize, BinaryBasicCollector<F>)>,
        binary_add_collector: Vec<(usize, BinaryAddCollector<F>)>,
        binary_extension_collector: Vec<(usize, BinaryExtensionCollector<F>)>,
        arith_collector: Vec<(usize, ArithInstanceCollector<F>)>,
        keccakf_collector: Vec<(usize, KeccakfCollector)>,
        sha256f_collector: Vec<(usize, Sha256fCollector)>,
        poseidon2_collector: Vec<(usize, Poseidon2Collector)>,
        arith_eq_collector: Vec<(usize, ArithEqCollector)>,
        arith_eq_384_collector: Vec<(usize, ArithEq384Collector)>,
        add256_collector: Vec<(usize, Add256Collector)>,
        dma_collector: Vec<(usize, DmaCollector)>,
        dma_pre_post_collector: Vec<(usize, DmaPrePostCollector)>,
        dma_64_aligned_collector: Vec<(usize, Dma64AlignedCollector)>,
        dma_unaligned_collector: Vec<(usize, DmaUnalignedCollector)>,
        rom_collector: Vec<(usize, RomCollector)>,
        arith_eq_inputs_generator: ArithEqCounterInputGen,
        arith_eq_384_inputs_generator: ArithEq384CounterInputGen,
        keccakf_inputs_generator: KeccakfCounterInputGen,
        sha256f_inputs_generator: Sha256fCounterInputGen,
        poseidon2_inputs_generator: Poseidon2CounterInputGen,
        arith_inputs_generator: ArithCounterInputGen,
        add256_inputs_generator: Add256CounterInputGen,
        dma_inputs_generator: DmaCounterInputGen,
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
            poseidon2_collector,
            arith_eq_collector,
            arith_eq_384_collector,
            add256_collector,
            dma_collector,
            dma_pre_post_collector,
            dma_64_aligned_collector,
            dma_unaligned_collector,
            rom_collector,
            arith_eq_inputs_generator,
            arith_eq_384_inputs_generator,
            keccakf_inputs_generator,
            sha256f_inputs_generator,
            poseidon2_inputs_generator,
            arith_inputs_generator,
            add256_inputs_generator,
            dma_inputs_generator,
            pending_transfers: VecDeque::with_capacity(64),
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
    fn route_data(&mut self, bus_id: BusId, data: &[PayloadType], data_ext: &[PayloadType]) {
        match bus_id {
            MEM_BUS_ID => {
                MemCollectorProcessor::new(&mut self.mem_collector, &mut self.mem_align_collector)
                    .process_mem_data(&data.try_into().unwrap());
            }
            OPERATION_BUS_ID => match data[OP_TYPE] {
                BINARY_TYPE => {
                    for (_, binary_add_collector) in &mut self.binary_add_collector {
                        binary_add_collector.process_data(&bus_id, data);
                    }

                    for (_, binary_basic_collector) in &mut self.binary_basic_collector {
                        binary_basic_collector.process_data(&bus_id, data);
                    }
                }
                BINARY_E_TYPE => {
                    for (_, binary_extension_collector) in &mut self.binary_extension_collector {
                        binary_extension_collector.process_data(&bus_id, data);
                    }
                }
                ARITH_TYPE => {
                    for (_, arith_collector) in &mut self.arith_collector {
                        arith_collector.process_data(&bus_id, data);
                    }

                    self.arith_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut self.pending_transfers,
                    );
                }
                KECCAK_TYPE => {
                    for (_, keccakf_collector) in &mut self.keccakf_collector {
                        keccakf_collector.process_data(&bus_id, data);
                    }

                    self.keccakf_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                SHA256_TYPE => {
                    for (_, sha256f_collector) in &mut self.sha256f_collector {
                        sha256f_collector.process_data(&bus_id, data);
                    }

                    self.sha256f_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                POSEIDON2_TYPE => {
                    for (_, poseidon2_collector) in &mut self.poseidon2_collector {
                        poseidon2_collector.process_data(&bus_id, data);
                    }
                    self.poseidon2_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                ARITH_EQ_TYPE => {
                    for (_, arith_eq_collector) in &mut self.arith_eq_collector {
                        arith_eq_collector.process_data(&bus_id, data);
                    }

                    self.arith_eq_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                ARITH_EQ_384_TYPE => {
                    for (_, arith_eq_384_collector) in &mut self.arith_eq_384_collector {
                        arith_eq_384_collector.process_data(&bus_id, data);
                    }

                    self.arith_eq_384_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                BIG_INT_OP_TYPE_ID => {
                    for (_, add256_collector) in &mut self.add256_collector {
                        add256_collector.process_data(&bus_id, data);
                    }

                    self.add256_inputs_generator.process_data(
                        &bus_id,
                        data,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                DMA_OP_TYPE_ID => {
                    for (_, dma_collector) in &mut self.dma_collector {
                        dma_collector.process_data(&bus_id, data, data_ext);
                    }
                    for (_, dma_pre_post_collector) in &mut self.dma_pre_post_collector {
                        dma_pre_post_collector.process_data(&bus_id, data, data_ext);
                    }
                    for (_, dma_64_aligned_collector) in &mut self.dma_64_aligned_collector {
                        dma_64_aligned_collector.process_data(&bus_id, data, data_ext);
                    }
                    for (_, dma_unaligned_collector) in &mut self.dma_unaligned_collector {
                        dma_unaligned_collector.process_data(&bus_id, data, data_ext);
                    }

                    self.dma_inputs_generator.process_data(
                        &bus_id,
                        data,
                        data_ext,
                        &mut MemCollectorProcessor::new(
                            &mut self.mem_collector,
                            &mut self.mem_align_collector,
                        ),
                    );
                }
                _ => {}
            },
            ROM_BUS_ID => {
                for (_, rom_collector) in &mut self.rom_collector {
                    rom_collector.process_data(&bus_id, data);
                }
            }
            _ => {}
        }
    }
}

impl<F: PrimeField64> DataBusTrait<PayloadType, Box<dyn BusDevice<PayloadType>>>
    for StaticDataBusCollect<PayloadType, F>
{
    #[inline(always)]
    fn write_to_bus(
        &mut self,
        bus_id: BusId,
        data: &[PayloadType],
        data_ext: &[PayloadType],
    ) -> bool {
        self.route_data(bus_id, data, data_ext);

        // Process all pending transfers in a batch to improve cache locality
        while let Some((pending_bus_id, pending_payload, pending_data_ext)) =
            self.pending_transfers.pop_front()
        {
            self.route_data(pending_bus_id, &pending_payload, &pending_data_ext);
        }

        true
    }

    fn on_close(&mut self) {}

    fn into_devices(
        self,
        _execute_on_close: bool,
    ) -> Vec<(Option<usize>, Option<Box<dyn BusDevice<PayloadType>>>)> {
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

        for (id, collector) in self.poseidon2_collector {
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

        for (id, collector) in self.dma_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.dma_pre_post_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.dma_64_aligned_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.dma_unaligned_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        for (id, collector) in self.rom_collector {
            result.push((Some(id), Some(Box::new(collector) as Box<dyn BusDevice<PayloadType>>)));
        }

        result
    }
}
