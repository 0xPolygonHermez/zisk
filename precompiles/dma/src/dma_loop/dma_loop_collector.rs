//! The collector of a `DmaLoop` instance (and of the loop block of a `CompactDma` one).

use std::any::Any;

use zisk_common::{BusDevice, BusId, ChunkId, DMA_ENCODED, OP, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_precomp_helpers::DmaInfo;

use crate::{DmaCollectorRoutingLog, DmaInputPosition, DmaLoopCollectCounters, DmaLoopInput};

pub struct DmaLoopCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaLoopInput>,
    /// Index of the input that ends the instance, swapped to the end when the inputs are taken.
    pub last_input_index: Option<usize>,

    pub chunk_id: ChunkId,

    /// Routing log for debugging and tracking collection operations.
    pub rlog: DmaCollectorRoutingLog,

    /// Upper bound on the inputs to collect.
    pub num_inputs: u64,

    /// Which rows of each class this instance takes.
    pub collect_counters: DmaLoopCollectCounters,
}

impl DmaLoopCollector {
    pub fn new(
        chunk_id: ChunkId,
        num_inputs: u64,
        collect_counters: DmaLoopCollectCounters,
    ) -> Self {
        Self {
            inputs: Vec::with_capacity(num_inputs as usize),
            last_input_index: None,
            chunk_id,
            rlog: DmaCollectorRoutingLog::new(chunk_id),
            num_inputs,
            collect_counters,
        }
    }

    /// Processes data received on the bus, collecting the loop part of the operations this
    /// instance was given.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        if self.inputs.len() == self.num_inputs as usize {
            debug_assert!(self.collect_counters.is_final_skip());
            return self.rlog.log_discard_cond(false, 1, data, false);
        }

        let op = data[OP] as u8;
        let encoded = data[DMA_ENCODED];
        let Some(class) = DmaLoopInput::class_of(op, encoded) else {
            self.rlog.log_discard(2, data);
            return true;
        };
        if DmaInfo::is_direct(encoded) && (op == ZiskOp::DMA_MEMCMP || op == ZiskOp::DMA_XMEMCMP) {
            panic!("Direct memcmp operations are not supported");
        }

        let rows = DmaLoopInput::total_rows(class, encoded) as u64;
        if let Some((skip, max_rows)) = self.collect_counters.should_collect(class, rows) {
            self.rlog.log_collect(rows as u32, data, skip, max_rows);
            self.add_input(DmaLoopInput::from(data, data_ext, skip as usize, max_rows as usize));
            if self.inputs.len() >= self.num_inputs as usize {
                debug_assert!(self.collect_counters.is_final_skip());
                return self.rlog.log_discard_cond(true, 11, data, false);
            }
        } else {
            self.rlog.log_discard(10, data);
        }
        true
    }

    /// Adds an input keeping the one that continues a previous instance first, and remembering
    /// the one that goes on in the next instance so it can be moved to the end.
    #[inline(always)]
    fn add_input(&mut self, input: DmaLoopInput) {
        let must_be_first = input.must_be_first();
        let must_be_last = input.must_be_last();
        let current_index = self.inputs.len();
        self.inputs.push(input);
        if must_be_first {
            if current_index > 0 {
                self.inputs.swap(0, current_index);
                // The one that was first moved to `current_index`.
                if self.last_input_index == Some(0) {
                    self.last_input_index = Some(current_index);
                }
            }
        } else if must_be_last {
            assert!(self.last_input_index.is_none(), "Multiple inputs marked as last input");
            self.last_input_index = Some(current_index);
        }
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

    pub fn take_inputs(&mut self) -> Vec<DmaLoopInput> {
        if let Some(last_index) = self.last_input_index.take() {
            let current_last_index = self.inputs.len() - 1;
            self.inputs.swap(last_index, current_last_index);
        }
        std::mem::take(&mut self.inputs)
    }
}

impl BusDevice<u64> for DmaLoopCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
