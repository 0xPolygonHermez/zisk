//! The `DmaUnalignedInstance` module defines an instance to perform the witness computation
//! for the Dma State Machine.
//!
//! It manages collected inputs and interacts with the `DmaSM` to compute witnesses for
//! execution plans.

use crate::{DmaCollectCounters, DmaCollectorRoutingLog, DmaInputPosition, DmaUnalignedInput};
use std::any::Any;
use zisk_common::{BusDevice, BusId, ChunkId, OP, OPERATION_BUS_ID, OP_TYPE};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};

pub struct DmaUnalignedCollector {
    /// Collected inputs for witness computation.
    pub inputs: Vec<DmaUnalignedInput>,
    pub last_input_index: Option<usize>,

    pub chunk_id: ChunkId,

    /// Routing log for debugging and tracking collection operations.
    pub rlog: DmaCollectorRoutingLog,

    /// The number of operations to collect.
    pub num_inputs: u64,

    /// Helper to skip instructions based on the plan's configuration.
    pub collect_counters: DmaCollectCounters,

    pub trace_offset: usize,
    pub last_segment_collector: bool,
}

impl DmaUnalignedCollector {
    /// Creates a new `DmaUnalignedCollector`.
    ///
    /// # Arguments
    ///
    /// * `bus_id` - The connected bus ID.
    /// * `num_inputs` - The number of inputs to collect.
    /// * `collect_counter` - The helper to skip instructions based on the plan's configuration.
    ///
    /// # Returns
    /// A new `DmaUnalignedCollector` instance initialized with the provided parameters.
    pub fn new(
        chunk_id: zisk_common::ChunkId,
        num_inputs: u64,
        collect_counters: DmaCollectCounters,
        last_segment_collector: bool,
    ) -> Self {
        Self {
            inputs: Vec::with_capacity(num_inputs as usize),
            num_inputs,
            collect_counters,
            trace_offset: 0,
            last_segment_collector,
            chunk_id,
            rlog: DmaCollectorRoutingLog::new(chunk_id),
            last_input_index: None,
        }
    }

    const DMA_UNALIGNED_OPS: [u8; 4] =
        [ZiskOp::DMA_MEMCPY, ZiskOp::DMA_XMEMCPY, ZiskOp::DMA_MEMCMP, ZiskOp::DMA_XMEMCMP];

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

        // Method get_count get the rows that applies, means that if a
        // input has src, dst aligned not applies
        let rows = DmaUnalignedInput::get_count(data) as u64;
        if rows == 0 {
            return true;
        }

        let op = data[OP] as u8;

        if !Self::DMA_UNALIGNED_OPS.contains(&op) {
            return true;
        }

        if self.inputs.len() == self.num_inputs as usize {
            self.collect_counters.debug_assert_is_final_skip();
            return self.rlog.log_discard_cond(false, 3, data, true);
        }

        if let Some((skip, max_count)) = self.collect_counters.should_collect(rows, op) {
            self.rlog.log_collect(rows as u32, data, skip, max_count);
            self.add_input(DmaUnalignedInput::from(
                data,
                data_ext,
                self.trace_offset,
                skip as usize,
                max_count as usize,
            ));

            self.trace_offset += max_count as usize;
            if self.inputs.len() >= self.num_inputs as usize {
                self.collect_counters.debug_assert_is_final_skip();
                self.rlog.log_discard(10, data);
                return false;
            }
        } else {
            self.rlog.log_discard(11, data);
        }
        true
    }

    /// Adds an input to the collector with proper ordering management.
    ///
    /// This method handles:
    /// - Adding the input to the vector
    /// - Managing inputs that must be first (swaps to position 0)
    /// - Tracking inputs that must be last (stores index for later swap)
    ///
    /// # Arguments
    /// * `input` - The input to add
    #[inline(always)]
    fn add_input(&mut self, input: DmaUnalignedInput) {
        // Check if input must be first before pushing
        let must_be_first = input.must_be_first();
        let must_be_last = input.must_be_last();
        let current_index = self.inputs.len();

        // Push the input
        self.inputs.push(input);

        // Handle ordering requirements
        if must_be_first {
            // Swap with position 0 if not already first
            if current_index > 0 {
                self.inputs.swap(0, current_index);
            }
        } else if must_be_last {
            // Edge case: if an input is huge and it's both first and last,
            // must_be_first takes precedence and this branch won't execute
            assert!(self.last_input_index.is_none(), "Multiple inputs marked as last input");
            self.last_input_index = Some(current_index);
        }
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
    pub fn take_inputs(&mut self) -> Vec<DmaUnalignedInput> {
        if let Some(last_index) = self.last_input_index {
            // If there's a last input index, swap it with the last element to ensure it's the last one in the trace.
            let current_last_index = self.inputs.len() - 1;
            self.inputs.swap(last_index, current_last_index);
        }
        std::mem::take(&mut self.inputs)
    }
    pub fn take_debug_inputs(&mut self) -> (String, Vec<DmaUnalignedInput>) {
        let debug_info = self.get_debug_info();
        let inputs = self.take_inputs();
        (debug_info, inputs)
    }
}

impl BusDevice<u64> for DmaUnalignedCollector {
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
