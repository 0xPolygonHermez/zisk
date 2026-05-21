//! `StaticDataBusCollect` — collector-phase data bus. See
//! [`static_data_bus`](crate::static_data_bus) for the counter-phase
//! counterpart.
use std::collections::VecDeque;

use data_bus::DataBusTrait;
use fields::PrimeField64;
use precomp_dma::Dma64AlignedCollector;
use precomp_dma::DmaCollector;
use precomp_dma::DmaCounterInputGen;
use precomp_dma::DmaPrePostCollector;
use precomp_dma::DmaUnalignedCollector;
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

use crate::error::{ExecutorError, ExecutorResult};
use crate::{BuiltinCollectors, PrecompileCollectors, StaticSMBundle};
use proofman_common::ProofCtx;
use std::collections::HashMap;
use zisk_common::Instance;

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
    /// ROM collector.
    pub rom_collector: Vec<(usize, RomCollector)>,

    /// Memory-related collectors.
    pub mem_collector: Vec<(usize, MemModuleCollector)>,
    /// Memory alignment collectors.
    pub mem_align_collector: Vec<(usize, MemAlignCollector)>,

    /// Arithmetic collectors.
    pub arith_collector: Vec<(usize, ArithInstanceCollector<F>)>,
    /// Arithmetic inputs generator.
    pub arith_inputs_generator: ArithCounterInputGen,

    /// Binary operation collectors.
    pub binary_basic_collector: Vec<(usize, BinaryBasicCollector<F>)>,
    /// Binary add operation collectors.
    pub binary_add_collector: Vec<(usize, BinaryAddCollector<F>)>,
    /// Binary extension operation collectors.
    pub binary_extension_collector: Vec<(usize, BinaryExtensionCollector<F>)>,

    /// Dma collectors.
    pub dma_collector: Vec<(usize, DmaCollector)>,
    /// Dma pre/post collectors.
    pub dma_pre_post_collector: Vec<(usize, DmaPrePostCollector)>,
    /// Dma 64-aligned collectors.
    pub dma_64_aligned_collector: Vec<(usize, Dma64AlignedCollector)>,
    /// Dma unaligned collectors.
    pub dma_unaligned_collector: Vec<(usize, DmaUnalignedCollector)>,
    /// Dma inputs generator.
    pub dma_inputs_generator: DmaCounterInputGen,

    /// Per-precompile collectors + input generators.
    pub precompiles: PrecompileCollectors<F>,

    /// Queue of pending data transfers to be processed.
    pending_transfers: VecDeque<(BusId, Vec<D>, Vec<D>)>,
}

const BINARY_TYPE: u64 = ZiskOperationType::Binary as u64;
const BINARY_E_TYPE: u64 = ZiskOperationType::BinaryE as u64;
const ARITH_TYPE: u64 = ZiskOperationType::Arith as u64;
const DMA_OP_TYPE_ID: u64 = ZiskOperationType::Dma as u64;

impl<F: PrimeField64> StaticDataBusCollect<PayloadType, F> {
    /// Constructs a collector-phase data bus for a single chunk. Each
    /// `global_idx` is dispatched to the matching built-in or
    /// precompile wrapper via `try_push_collector`; on a miss the
    /// air-id is reported. Callers are expected to skip empty chunks
    /// (no `global_idxs`) themselves — this constructor always builds.
    /// Mirrors `StaticDataBus::from_bundle` on the counter side.
    pub fn for_chunk(
        bundle: &StaticSMBundle<F>,
        pctx: &ProofCtx<F>,
        instances: &HashMap<usize, &dyn Instance<F>>,
        chunk_id: usize,
        global_idxs: &[usize],
    ) -> ExecutorResult<Self> {
        let mut builtins = BuiltinCollectors::start_chunk(bundle)?;
        let mut precompiles = PrecompileCollectors::start_chunk(bundle)?;

        for global_idx in global_idxs {
            let global_id = *global_idx;
            let instance =
                instances.get(&global_id).ok_or(ExecutorError::InstanceNotFound { global_id })?;
            let (airgroup_id, air_id) = pctx
                .dctx_get_instance_info(global_id)
                .map_err(|source| ExecutorError::InstanceInfo { global_id, source })?;

            let pushed = builtins.try_push_collector(air_id, *instance, chunk_id, global_id)?
                || precompiles.try_push_collector(air_id, *instance, chunk_id, global_id)?;

            if !pushed {
                return Err(ExecutorError::StateMachineNotFound { airgroup_id, air_id });
            }
        }

        Ok(Self {
            rom_collector: builtins.rom,
            mem_collector: builtins.mem,
            mem_align_collector: builtins.mem_align,
            arith_collector: builtins.arith,
            arith_inputs_generator: builtins.arith_inputs_generator,
            binary_basic_collector: builtins.binary_basic,
            binary_add_collector: builtins.binary_add,
            binary_extension_collector: builtins.binary_extension,
            dma_collector: builtins.dma,
            dma_pre_post_collector: builtins.dma_pre_post,
            dma_64_aligned_collector: builtins.dma_64_aligned,
            dma_unaligned_collector: builtins.dma_unaligned,
            dma_inputs_generator: builtins.dma_inputs_generator,
            precompiles,
            pending_transfers: VecDeque::with_capacity(64),
        })
    }

    /// Routes data to the devices subscribed to a specific bus ID or global devices.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus to route the data to.
    /// * `data` - A reference to the data payload being routed.
    /// * `data_ext` - A reference to the extended data payload being routed.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    fn route_data(&mut self, bus_id: BusId, data: &[PayloadType], data_ext: &[PayloadType]) {
        match bus_id {
            MEM_BUS_ID => {
                MemCollectorProcessor::new(&mut self.mem_collector, &mut self.mem_align_collector)
                    .process_mem_data(
                        &data
                            .try_into()
                            .expect("MEM_BUS_ID payload must have the correct array length"),
                    );
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
                op => {
                    self.precompiles.dispatch_op(
                        op as u32,
                        &bus_id,
                        data,
                        &mut self.mem_collector,
                        &mut self.mem_align_collector,
                    );
                }
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

        result.extend(self.precompiles.into_device_entries());

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
