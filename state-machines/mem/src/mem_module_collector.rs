use std::collections::VecDeque;

use crate::{mem_bus_data_to_input::MemBusDataToInput, MemInput, MemPreviousSegment};
use mem_common::MemModuleCheckPoint;
use zisk_common::{BusDevice, BusId, SegmentId, MEM_BUS_ID};

#[derive(Debug)]
pub struct MemModuleCollector {
    /// Binary Basic state machine
    pub mem_check_point: MemModuleCheckPoint,

    /// Collected inputs
    pub inputs: Vec<MemInput>,
    pub prev_segment: Option<MemPreviousSegment>,
    pub min_addr: u32,
    pub _segment_id: SegmentId,
    pub count: u32,
    pub to_count: u32,
    pub skip: u32,
    pub is_first_chunk_of_segment: bool,
}

impl MemModuleCollector {
    pub fn new(
        mem_check_point: &MemModuleCheckPoint,
        min_addr: u32,
        segment_id: SegmentId,
        is_first_chunk_of_segment: bool,
    ) -> Self {
        // let prev_addr = mem_check_point.reference_addr;
        // let prev_step = MemHelpers::first_chunk_mem_step(mem_check_point.reference_addr_chunk);
        let count = mem_check_point.count;
        let to_count = mem_check_point.to_count;
        let skip = mem_check_point.from_skip;
        Self {
            inputs: Vec::new(),
            mem_check_point: mem_check_point.clone(),
            prev_segment: None,
            min_addr,
            _segment_id: segment_id,
            count,
            to_count,
            skip,
            is_first_chunk_of_segment,
        }
    }

    /// Discards the given memory access if it is not part of the current segment.
    ///
    /// This function checks whether the given memory access (defined by `addr`, `step`, and `value`)
    /// should be discarded. If the access is not part of the current segment, the function returns `true`.
    ///
    /// # Parameters
    /// - `addr`: The memory address (8 bytes aligned).
    /// - `step`: The mem_step of the memory access.
    /// - `value`: The value to be read or written.
    ///
    /// # Returns
    /// `true` if the access should be discarded, `false` otherwise.
    fn discart_addr_step(&mut self, addr_w: u32, step: u64, value: u64) -> bool {
        // Check if the address is out of the range of the current checkpoint, or
        // out of memory area.
        if addr_w > self.mem_check_point.to_addr || addr_w < self.min_addr {
            return true;
        }

        if addr_w < self.mem_check_point.from_addr {
            return true;
        }

        if addr_w == self.mem_check_point.from_addr && self.skip > 0 {
            if self.skip == 1 && self.is_first_chunk_of_segment {
                // The last discart before accept, we need to store the previous segment data
                self.prev_segment = Some(MemPreviousSegment { addr: addr_w, step, value });
            }

            self.skip -= 1;
            return true;
        }

        if self.count == 0 || (addr_w == self.mem_check_point.to_addr && self.to_count == 0) {
            return true;
        }

        // from_addr <= addr <= to_addr
        if addr_w == self.mem_check_point.to_addr {
            self.to_count -= 1;
        }

        self.count -= 1;
        false
    }

    /// Pushes the access only if this access must be managed with current instance.
    ///
    /// This function checks whether the given memory access (defined by `addr_w`, `step`, and `value`)
    /// should be discarded using `discard_addr_step()`. If it is not discarded,
    /// a new `MemInput` instance is created and pushed to `inputs`.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (8 bytes aligned).
    /// - `step`: The mem_step of the memory access.
    /// - `is_write`: Indicates whether the access is a write operation.
    /// - `value`: The value to be read or written.
    fn filtered_inputs_push(&mut self, inputs: Vec<MemInput>) {
        for input in inputs {
            if !self.discart_addr_step(input.addr, input.step, input.value) {
                self.inputs.push(input);
            }
        }
    }
}

impl BusDevice<u64> for MemModuleCollector {
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) -> bool {
        debug_assert!(*bus_id == MEM_BUS_ID);

        let inputs = MemBusDataToInput::bus_data_to_input(data);
        self.filtered_inputs_push(inputs);

        true
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
