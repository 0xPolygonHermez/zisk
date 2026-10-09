//! The `DmaWithPrePostCollector` module defines a collector to gather all inputs for an instance
//! of the fused DmaWithPrePost State Machine.

use std::any::Any;

use zisk_common::{BusDevice, BusId, ChunkId, DMA_ENCODED, OP, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_precomp_helpers::DmaInfo;

use crate::{DmaCollectorRoutingLog, DmaWithPrePostCollectCounters, DmaWithPrePostInput};

pub struct DmaWithPrePostCollector {
    /// The chunk identifier being collected (used for tracing/debugging).
    pub chunk_id: ChunkId,

    /// Collected inputs for witness computation, one per DMA operation.
    pub inputs: Vec<DmaWithPrePostInput>,

    /// Routing log for debugging and tracking collection operations.
    pub rlog: DmaCollectorRoutingLog,

    /// The number of operations to collect.
    pub num_ops: u64,

    /// Counters to determine which operations to collect based on the plan's configuration.
    pub collect_counters: DmaWithPrePostCollectCounters,
}

impl DmaWithPrePostCollector {
    pub fn new(
        chunk_id: ChunkId,
        num_ops: u64,
        collect_counters: DmaWithPrePostCollectCounters,
    ) -> Self {
        Self {
            chunk_id,
            inputs: Vec::with_capacity(num_ops as usize),
            num_ops,
            collect_counters,
            rlog: DmaCollectorRoutingLog::new(chunk_id),
        }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// Unlike the `Dma` / `DmaPrePost` collectors, this one takes or leaves a **whole operation**:
    /// its PRE row has to be adjacent to its DMA row, so it can never be shared between two
    /// instances. The plan therefore counts operations, split by row cost — see
    /// [`DmaWithPrePostCollectCounters`].
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        if self.inputs.len() == self.num_ops as usize {
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

        let is_double = DmaWithPrePostInput::is_double(encoded);
        if self.collect_counters.should_collect(is_double) {
            let rows = if is_double {
                DmaWithPrePostInput::DOUBLE_ROW
            } else {
                DmaWithPrePostInput::SINGLE_ROW
            };
            self.rlog.log_collect(rows as u32, data, 0, 0);
            self.inputs.push(DmaWithPrePostInput::from_op(data, data_ext));
            if self.inputs.len() >= self.num_ops as usize {
                debug_assert!(self.collect_counters.is_final_skip());
                self.rlog.log_discard(4, data);
                return true;
            }
        } else {
            self.rlog.log_discard(3, data);
        }

        true
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

impl BusDevice<u64> for DmaWithPrePostCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
