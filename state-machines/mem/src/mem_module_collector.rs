use crate::{
    mem_bus_data_to_input::MemBusDataToInput, MemInput, MemModuleCheckPoint, MemPreviousSegment,
};
use data_bus::{BusDevice, BusId, MEM_BUS_ID};
use zisk_common::SegmentId;

#[derive(Debug)]
pub struct MemModuleCollector {
    /// Binary Basic state machine
    pub mem_check_point: MemModuleCheckPoint,

    /// Collected inputs
    pub inputs: Vec<MemInput>,
    pub prev_segment: Option<MemPreviousSegment>,
    pub min_addr: u32,
    pub segment_id: SegmentId,
    pub count: u32,
    pub to_count: u32,
    pub skip: u32,
}

impl MemModuleCollector {
    pub fn new(
        mem_check_point: &MemModuleCheckPoint,
        min_addr: u32,
        segment_id: SegmentId,
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
            segment_id,
            count,
            to_count,
            skip,
        }
    }

    fn debug_discard(&self, reason: u8, addr_w: u32, step: u64, value: u64) {
        // let label = if reason == 0 { "ACCEPT" } else { &format!("DISCARD{}", reason) };
        // println!(
        //     "[Mem] {} discard_addr_step [0x{:X},{}] {} [F:0x{:X},{}/{} T:0x{:X},{}/{} C:{}/{}]",
        //     label,
        //     addr_w * 8,
        //     step,
        //     value,
        //     self.mem_check_point.from_addr * 8,
        //     self.skip,
        //     self.mem_check_point.from_skip,
        //     self.mem_check_point.to_addr * 8,
        //     self.to_count,
        //     self.mem_check_point.to_count,
        //     self.count,
        //     self.mem_check_point.count,
        // );
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
        if addr_w > self.mem_check_point.to_addr || addr_w < self.min_addr {
            self.debug_discard(1, addr_w, step, value);
            return true;
        }

        if addr_w < self.mem_check_point.from_addr {
            self.debug_discard(2, addr_w, step, value);
            return true;
        }

        if addr_w == self.mem_check_point.from_addr && self.skip > 0 {
            if self.skip == 1 && self.mem_check_point.is_first_chunk() {
                // The last discart before accept, we need to store the previous segment data
                self.prev_segment = Some(MemPreviousSegment { addr: addr_w, step, value });
                self.debug_discard(3, addr_w, step, value);
            } else {
                self.debug_discard(4, addr_w, step, value);
            }

            self.skip -= 1;
            return true;
        }

        if self.count == 0 || (addr_w == self.mem_check_point.to_addr && self.to_count == 0) {
            self.debug_discard(5, addr_w, step, value);
            return true;
        }

        // from_addr <= addr <= to_addr
        if addr_w == self.mem_check_point.to_addr {
            self.to_count -= 1;
        }

        self.count -= 1;
        self.debug_discard(0, addr_w, step, value);
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
    fn process_data(&mut self, bus_id: &BusId, data: &[u64]) -> Option<Vec<(BusId, Vec<u64>)>> {
        debug_assert!(*bus_id == MEM_BUS_ID);

        let inputs = MemBusDataToInput::bus_data_to_input(data);
        self.filtered_inputs_push(inputs);

        None
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![MEM_BUS_ID]
    }

    /// Provides a dynamic reference for downcasting purposes.
    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
