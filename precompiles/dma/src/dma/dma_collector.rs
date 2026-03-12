//! The `DmaCollector` module defines a collector to gather all inputs for an instance
//! of the DMA State Machine.

use std::any::Any;

use precompiles_helpers::DmaInfo;
use zisk_common::{BusDevice, BusId, ChunkId, DMA_ENCODED, OP, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

use crate::{DmaCollectCounters, DmaCollectorRoutingLog, DmaInput};

pub struct DmaCollector {
    /// The chunk identifier being collected (used for tracing/debugging).
    pub chunk_id: ChunkId,

    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaInput>,

    /// Routing log for debugging and tracking collection operations.
    pub rlog: DmaCollectorRoutingLog,

    /// The number of operations to collect.
    pub num_operations: u64,

    /// Counters to determine which operations to collect based on the plan's configuration.
    pub collect_counters: DmaCollectCounters,
}

impl DmaCollector {
    /// Creates a new `DmaCollector`.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The chunk identifier for this collector instance.
    /// * `num_operations` - The number of operations to collect.
    /// * `collect_counters` - Counters to determine which operations to collect based on the plan's configuration.
    ///
    /// # Returns
    /// A new `DmaCollector` instance initialized with the provided parameters.
    pub fn new(
        chunk_id: ChunkId,
        num_operations: u64,
        collect_counters: DmaCollectCounters,
    ) -> Self {
        Self {
            chunk_id,
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_counters,
            rlog: DmaCollectorRoutingLog::new(chunk_id),
        }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus (validated to be OPERATION_BUS_ID).
    /// * `data` - The main data array received from the bus containing operation information.
    /// * `data_ext` - Extended data array containing additional operation-specific information.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        if self.inputs.len() == self.num_operations as usize {
            debug_assert!(self.collect_counters.is_final_skip());
            return self.rlog.log_discard_cond(false, 1, data, false);
        }

        let encoded = data[DMA_ENCODED];
        let op = data[OP] as u8;
        if DmaInfo::is_direct(encoded) {
            if op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP {
                // We need to collect all memcmp/memcpy operations for the pre/post processing.
                panic!("Direct memcmp/memcpy operations are not supported");
            }
            self.rlog.log_discard(2, data);
            return true;
        }

        if self.collect_counters.should_collect_single_row(op) {
            self.rlog.log_collect(1, data, 0, 0);
            self.inputs.push(if op == ZiskOp::DMA_XMEMSET {
                DmaInput::from_memset(encoded, op, data, data_ext)
            } else {
                DmaInput::from(encoded, op, data, data_ext)
            });
            if self.inputs.len() >= self.num_operations as usize {
                debug_assert!(self.collect_counters.is_final_skip());
                self.rlog.log_discard(4, data);
                return true;
            }
        } else {
            self.rlog.log_discard(3, data);
        }

        true
    }
    /// Returns debug information about the collector's state.
    ///
    /// When the `save_dma_collectors` feature is enabled, this returns detailed information
    /// including chunk ID, number of collected inputs, counter information, and routing log.
    /// Otherwise, returns an empty string.
    ///
    /// # Returns
    /// A formatted string with debug information.
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

impl BusDevice<u64> for DmaCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
