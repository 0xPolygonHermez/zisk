//! The `DmaCompactCollector` gathers, for one chunk, the DMA operations that a `DmaCompact`
//! instance proves: the plan names them by position in the bus order of the chunk (see
//! [`DmaCompactChunkPlan`]).

use std::any::Any;

use zisk_common::{BusDevice, BusId, ChunkId, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::ZiskOperationType;

use crate::{DmaCompactChunkPlan, DmaCompactInput};

pub struct DmaCompactCollector {
    pub chunk_id: ChunkId,

    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaCompactInput>,

    /// Which operations of the chunk to collect.
    pub plan: DmaCompactChunkPlan,

    /// DMA operations of the chunk seen so far.
    seen: u32,
}

impl DmaCompactCollector {
    pub fn new(chunk_id: ChunkId, plan: DmaCompactChunkPlan) -> Self {
        Self { chunk_id, inputs: Vec::with_capacity(plan.count_ops as usize), plan, seen: 0 }
    }

    /// Processes data received on the bus, collecting the inputs necessary for witness computation.
    ///
    /// # Returns
    /// `true` while there may be more operations to collect, `false` once every planned operation
    /// has been collected.
    #[inline(always)]
    pub fn process_data(&mut self, bus_id: &BusId, data: &[u64], data_ext: &[u64]) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] != ZiskOperationType::Dma as u64 {
            return true;
        }

        let index = self.seen;
        self.seen += 1;

        if index < self.plan.skip_ops {
            return true;
        }
        if self.inputs.len() >= self.plan.count_ops as usize {
            return false;
        }

        let input = DmaCompactInput::from(data, data_ext);
        debug_assert!(
            input.rows() <= self.plan.rows as usize,
            "DmaCompactCollector: operation of {} rows exceeds the {} rows planned for chunk {}",
            input.rows(),
            self.plan.rows,
            self.chunk_id
        );
        self.inputs.push(input);

        self.inputs.len() < self.plan.count_ops as usize
    }

    pub fn get_debug_info(&self) -> String {
        format!(
            "CC|{}|{}|ops:({}){}|rows:{}\n",
            self.chunk_id,
            self.inputs.len(),
            self.plan.skip_ops,
            self.plan.count_ops,
            self.plan.rows
        )
    }
}

impl BusDevice<u64> for DmaCompactCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
