//! The `DmaPrePostCollector` module defines an collector to calculate all inputs of an instance
//! for the DmaPrePost State Machine.

use std::any::Any;

use precompiles_helpers::DmaInfo;
use zisk_common::{BusDevice, BusId, ChunkId, DMA_ENCODED, OP, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use crate::{DmaCollectCounters, DmaCollectorRoutingLog, DmaPrePostInput};

pub struct DmaPrePostCollector {
    pub chunk_id: ChunkId,
    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaPrePostInput>,

    /// Routing log for debugging and tracking collection operations.
    pub rlog: DmaCollectorRoutingLog,

    /// The number of operations to collect.
    pub num_inputs: u64,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_counters: DmaCollectCounters,
}

impl DmaPrePostCollector {
    /// Creates a new `DmaPrePostCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_inputs` - The number of inputs to collect.
    /// * `collect_skipper` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `DmaPrePostCollector` instance initialized with the provided parameters.
    pub fn new(chunk_id: ChunkId, num_inputs: u64, collect_counters: DmaCollectCounters) -> Self {
        Self {
            chunk_id,
            inputs: Vec::with_capacity(num_inputs as usize),
            num_inputs,
            collect_counters,
            rlog: DmaCollectorRoutingLog::new(chunk_id),
        }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `data` - The data received from the bus.
    /// * `pending` – A queue of pending bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A tuple where:
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        if self.inputs.len() == self.num_inputs as usize {
            return self.rlog.log_discard_cond(false, 1, data, false);
        }

        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        if DmaInfo::is_direct(encoded) {
            if op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP {
                // We need to collect all memcmp/memcpy operations for the pre/post processing.
                panic!("Direct memcmp/memcpy operations are not supported");
            }
            self.rlog.log_discard(2, data);
            return true;
        }

        let rows = DmaInfo::get_pre_writes(encoded);
        if rows == 0 {
            self.rlog.log_discard(3, data);
            return true;
        }

        if let Some((skip, max_count)) = self.collect_counters.should_collect(rows as u64, op) {
            self.rlog.log_collect(rows as u32, data, skip, max_count);
            self.inputs.extend(match op {
                ZiskOp::DMA_XMEMSET => {
                    DmaPrePostInput::from_memset(data, data_ext, skip, max_count)
                }
                ZiskOp::DMA_MEMCMP | ZiskOp::DMA_XMEMCMP => {
                    DmaPrePostInput::from(data, data_ext, skip, max_count)
                }
                ZiskOp::DMA_INPUTCPY | ZiskOp::DMA_MEMCPY | ZiskOp::DMA_XMEMCPY => {
                    DmaPrePostInput::from(data, data_ext, skip, max_count)
                }
                _ => panic!("Invalid operation 0x{op:02X}"),
            });
        } else {
            self.rlog.log_discard(10, data);
        }
        self.rlog.log_discard_cond(self.inputs.len() < self.num_inputs as usize, 13, data, true)
    }

    pub fn get_debug_info(&self) -> String {
        #[cfg(feature = "save_dma_collectors")]
        return format!(
            "CC|{}|{}|{}\n",
            self.chunk_id,
            self.inputs.len(),
            self.collect_counters.get_debug_info(),
        ) + &self.rlog.get_debug_info();
        #[cfg(not(feature = "save_dma_collectors"))]
        String::new()
    }
}

impl BusDevice<u64> for DmaPrePostCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
