//! The `DmaCounter` module defines a counter for tracking dma-related operations
//! sent over the data bus. It connects to the bus and gathers metrics for specific
//! `ZiskOperationType::Dma` instructions.

use precompiles_common::MemProcessor;
use precompiles_helpers::DmaInfo;
use std::ops::Add;
use zisk_common::{BusDevice, BusDeviceMode, BusId, Metrics, B, OPERATION_BUS_ID, OP_TYPE};
use zisk_common::{A, OPERATION_PRECOMPILED_BUS_DATA_SIZE};
use zisk_core::ZiskOperationType;

use crate::{generate_dma_mem_inputs, skip_dma_mem_inputs, DMA_64_ALIGNED_OPS_BY_ROW};

/// The `DmaCounter` struct represents a counter that monitors and measures
/// dma-related operations on the data bus.
///
/// It tracks specific operation types (`ZiskOperationType`) and updates counters for each
/// accepted operation type whenever data is processed on the bus.
#[derive(Debug)]
pub struct DmaCounterInputGen {
    /// sizes of memcpy
    pub dma_pre_post_ops: usize,
    pub dma_ops: usize,
    pub dma_unaligned_inputs: usize,
    pub dma_unaligned_rows: usize,
    pub dma_64_aligned_inputs: usize,
    pub dma_64_aligned_rows: usize,

    /// Bus device mode (counter or input generator).
    pub mode: BusDeviceMode,
}

impl DmaCounterInputGen {
    /// Creates a new instance of `DmaCounter`.
    ///
    /// # Arguments
    /// * `mode` - The ID of the bus to which this counter is connected.
    ///
    /// # Returns
    /// A new `DmaCounter` instance.
    pub fn new(mode: BusDeviceMode) -> Self {
        Self {
            dma_pre_post_ops: 0,
            dma_ops: 0,
            dma_unaligned_inputs: 0,
            dma_64_aligned_inputs: 0,
            dma_unaligned_rows: 0,
            dma_64_aligned_rows: 0,
            mode,
        }
    }

    /// Retrieves the count of instructions for a specific `ZiskOperationType`.
    ///
    /// # Arguments
    /// * `dst` - The destination address of operation.
    /// * `src` - The source address of operation.
    /// * `count` - The bytes of operation.
    pub fn inst_count_memcpy(&mut self, dst: u64, src: u64, count: usize) {
        let dst_offset = dst & 0x07;
        let src_offset = src & 0x07;

        // offset => max bytes is 8 - offset
        if count > 0 {
            let remaining = if dst_offset > 0 {
                self.dma_pre_post_ops += 1;
                count - std::cmp::min(8 - dst_offset as usize, count)
            } else {
                count
            };

            if (remaining % 8) > 0 {
                // adding a post because last write isn't full (8-bytes)
                self.dma_pre_post_ops += 1;
            }
            if dst_offset == src_offset {
                // println!(
                //     "count: {count} remaining: {remaining} self.dma_64_aligned_ops: {}",
                //     self.dma_64_aligned_rows
                // );
                let rows = (remaining >> 3).div_ceil(DMA_64_ALIGNED_OPS_BY_ROW);
                self.dma_64_aligned_rows += rows;
                self.dma_64_aligned_inputs += 1;
            } else if remaining > 7 {
                // check remaining because unaligned add an extra row for each unaligned, means
                // if remaming >> 3 is 0, add extra row.
                // on unalignmed_ops, each dst write use its src read and next src read also.
                // the last src read don't have write.
                self.dma_unaligned_rows += (remaining >> 3) + 1;
                self.dma_unaligned_inputs += 1;
            }
        }
        self.dma_ops += 1;
    }

    /// Processes data received on the bus, updating counters and generating inputs when applicable.
    ///
    /// # Arguments
    /// * `bus_id` - The ID of the bus sending the data.
    /// * `data` - The data received from the bus.
    /// * `mem_processors` – A queue of mem_processors bus operations used to send derived inputs.
    ///
    /// # Returns
    /// A boolean indicating whether the program should continue execution or terminate.
    /// Returns `true` to continue execution, `false` to stop.
    #[inline(always)]
    pub fn process_data<P: MemProcessor>(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        data_ext: &[u64],
        mem_processors: &mut P,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if data[OP_TYPE] as u32 != ZiskOperationType::Dma as u32 {
            return true;
        }

        match self.mode {
            BusDeviceMode::Counter => {
                self.measure(data);
                generate_dma_mem_inputs(data, data_ext, true, mem_processors);
            }
            BusDeviceMode::CounterAsm => {
                self.measure(data);
            }
            BusDeviceMode::InputGenerator => {
                if skip_dma_mem_inputs(data, data_ext, mem_processors) {
                    return true;
                }
                generate_dma_mem_inputs(data, data_ext, false, mem_processors);
            }
        }

        true
    }
}

impl Metrics for DmaCounterInputGen {
    /// Tracks activity on the connected bus and updates counters for recognized operations.
    ///
    /// # Arguments
    /// * `_bus_id` - The ID of the bus (unused in this implementation).
    /// * `_data` - The data received from the bus.
    ///
    /// # Returns
    /// An empty vector, as this implementation does not produce any derived inputs for the bus.
    #[inline(always)]
    fn measure(&mut self, data: &[u64]) {
        let dst = data[A];
        let src = data[B];
        let count = DmaInfo::get_count(data[OPERATION_PRECOMPILED_BUS_DATA_SIZE]);
        self.inst_count_memcpy(dst, src, count);
    }

    /// Provides a dynamic reference for downcasting purposes.
    ///
    /// # Returns
    /// A reference to `self` as `dyn std::any::Any`.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for DmaCounterInputGen {
    type Output = DmaCounterInputGen;

    /// Combines two `DmaCounter` instances by summing their counters.
    ///
    /// # Arguments
    /// * `self` - The first `DmaCounter` instance.
    /// * `other` - The second `DmaCounter` instance.
    ///
    /// # Returns
    /// A new `DmaCounter` with combined counters.
    fn add(self, other: Self) -> DmaCounterInputGen {
        DmaCounterInputGen {
            dma_pre_post_ops: self.dma_pre_post_ops + other.dma_pre_post_ops,
            dma_ops: self.dma_ops + other.dma_ops,
            dma_unaligned_inputs: self.dma_unaligned_inputs + other.dma_unaligned_inputs,
            dma_64_aligned_inputs: self.dma_64_aligned_inputs + other.dma_64_aligned_inputs,
            dma_unaligned_rows: self.dma_unaligned_rows + other.dma_unaligned_rows,
            dma_64_aligned_rows: self.dma_64_aligned_rows + other.dma_64_aligned_rows,
            mode: self.mode,
        }
    }
}

impl BusDevice<u64> for DmaCounterInputGen {
    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
