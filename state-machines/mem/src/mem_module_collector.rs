use std::collections::VecDeque;

use crate::{MemInput, MemPreviousSegment};
use mem_common::{MemHelpers, MemModuleCheckPoint};
use zisk_common::{BusDevice, BusId, MemBusData, MemCollectorInfo, SegmentId, MEM_BUS_ID};

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
            inputs: Vec::with_capacity(count as usize),
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

    /// Processes an unaligned memory access.
    ///
    /// Processes an unaligned memory access by computing all necessary aligned memory operations
    /// required to validate the unaligned access. The method determines the specific access case
    /// based on whether it involves a single or double memory access and calls the appropriate
    /// method to handle it.
    ///
    /// # Parameters
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_data(&mut self, data: &[u64]) {
        let addr = MemBusData::get_addr(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let bytes = MemBusData::get_bytes(data);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        if MemHelpers::is_double(addr, bytes) {
            if is_write {
                self.process_unaligned_double_write(addr_w, bytes, data);
            } else {
                self.process_unaligned_double_read(addr_w, data);
            }
        } else if is_write {
            self.process_unaligned_single_write(addr_w, bytes, data);
        } else {
            self.process_unaligned_single_read(addr_w, data);
        }
    }

    /// Processes an unaligned single read operation.
    ///
    /// Handles an unaligned single read operation by computing all necessary aligned memory
    /// operations required to validate the unaligned access. Finally, it uses `filtered_inputs_push`
    /// to push only the necessary memory accesses into `inputs`.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (aligned to 8 bytes).
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_single_read(&mut self, addr_w: u32, data: &[u64]) {
        let value = MemBusData::get_mem_values(data)[0];
        let step = MemBusData::get_step(data);
        if !self.discart_addr_step(addr_w, step, value) {
            self.inputs.push(MemInput::new(addr_w, false, step, value));
        }
    }

    /// Processes an unaligned single write operation.
    ///
    /// Handles an unaligned single write operation by computing all necessary aligned memory
    /// operations required to validate the unaligned access. Additionally, it calculates the
    /// write value for the given memory access. Finally, it uses `filtered_inputs_push` to
    /// push only the necessary memory accesses into `inputs`.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (aligned to 8 bytes).
    /// - `bytes`: The number of bytes to be written.
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_single_write(&mut self, addr_w: u32, bytes: u8, data: &[u64]) {
        let read_values = MemBusData::get_mem_values(data);
        let write_values = MemHelpers::get_write_values(
            MemBusData::get_addr(data),
            bytes,
            MemBusData::get_value(data),
            read_values,
        );
        let step = MemBusData::get_step(data);
        let read_step = MemHelpers::get_read_step(step);
        let write_step = MemHelpers::get_write_step(step);
        if !self.discart_addr_step(addr_w, read_step, read_values[0]) {
            self.inputs.push(MemInput::new(addr_w, false, read_step, read_values[0]));
        }
        if !self.discart_addr_step(addr_w, write_step, write_values[0]) {
            self.inputs.push(MemInput::new(addr_w, true, write_step, write_values[0]));
        }
    }

    /// Processes an unaligned double read operation.
    ///
    /// Handles an unaligned double read operation by computing all necessary aligned memory
    /// operations required to validate the unaligned access. Finally, it uses `filtered_inputs_push`
    /// to push only the necessary memory accesses into `inputs`.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (aligned to 8 bytes).
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_double_read(&mut self, addr_w: u32, data: &[u64]) {
        let read_values = MemBusData::get_mem_values(data);
        let step = MemBusData::get_step(data);
        if !self.discart_addr_step(addr_w, step, read_values[0]) {
            self.inputs.push(MemInput::new(addr_w, false, step, read_values[0]));
        }

        if !self.discart_addr_step(addr_w + 1, step, read_values[1]) {
            self.inputs.push(MemInput::new(addr_w + 1, false, step, read_values[1]));
        }
    }

    /// Processes an unaligned double write operation.
    ///
    /// Handles an unaligned double write operation by computing all necessary aligned memory
    /// operations required to validate the unaligned access. Additionally, it calculates the
    /// write value for the given memory access. Finally, it uses `filtered_inputs_push` to
    /// push only the necessary memory accesses into `inputs`.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (aligned to 8 bytes).
    /// - `bytes`: The number of bytes to be written.
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_double_write(&mut self, addr_w: u32, bytes: u8, data: &[u64]) {
        let read_values = MemBusData::get_mem_values(data);
        let write_values = MemHelpers::get_write_values(
            MemBusData::get_addr(data),
            bytes,
            MemBusData::get_value(data),
            read_values,
        );
        let step = MemBusData::get_step(data);
        let read_step = MemHelpers::get_read_step(step);
        let write_step = MemHelpers::get_write_step(step);

        // IMPORTANT: inputs must be ordered by step
        if !self.discart_addr_step(addr_w, read_step, read_values[0]) {
            self.inputs.push(MemInput::new(addr_w, false, read_step, read_values[0]));
        }

        if !self.discart_addr_step(addr_w + 1, read_step, read_values[1]) {
            self.inputs.push(MemInput::new(addr_w + 1, false, read_step, read_values[1]));
        }

        if !self.discart_addr_step(addr_w, write_step, write_values[0]) {
            self.inputs.push(MemInput::new(addr_w, true, write_step, write_values[0]));
        }

        if !self.discart_addr_step(addr_w + 1, write_step, write_values[1]) {
            self.inputs.push(MemInput::new(addr_w + 1, true, write_step, write_values[1]));
        }
    }

    pub fn bus_data_to_input(&mut self, data: &[u64]) {
        // decoding information in bus

        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);

        // If the access is unaligned (not aligned to 8 bytes or has a width different from 8 bytes).

        if !MemHelpers::is_aligned(addr, bytes) {
            self.process_unaligned_data(data);
        } else {
            // Direct case when is aligned, calculated 8 bytes addres (addr_w) and check if is a
            // write or read.

            let step = MemBusData::get_step(data);
            let addr_w = MemHelpers::get_addr_w(addr);
            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            if is_write {
                let value = MemBusData::get_value(data);
                if !self.discart_addr_step(addr_w, step, value) {
                    self.inputs.push(MemInput::new(addr_w, true, step, value));
                }
            } else {
                let value = MemBusData::get_mem_values(data)[0];
                if !self.discart_addr_step(addr_w, step, value) {
                    self.inputs.push(MemInput::new(addr_w, false, step, value));
                }
            }
        }
    }

    pub fn get_mem_collector_info(&self) -> MemCollectorInfo {
        MemCollectorInfo {
            from_addr: self.mem_check_point.from_addr,
            to_addr: self.mem_check_point.to_addr,
            min_addr: self.min_addr,
        }
    }
}

impl BusDevice<u64> for MemModuleCollector {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == MEM_BUS_ID);

        if self.count == 0 {
            return false;
        }

        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        let is_unaligned = !MemHelpers::is_aligned(addr, bytes);
        let unaligned_double = is_unaligned && MemHelpers::is_double(addr, bytes);

        let addr_w = MemHelpers::get_addr_w(addr);

        if !unaligned_double
            && (addr_w > self.mem_check_point.to_addr
                || addr_w < self.min_addr
                || addr_w < self.mem_check_point.from_addr)
        {
            return true;
        }

        if unaligned_double
            && (addr_w > self.mem_check_point.to_addr
                || addr_w < self.min_addr
                || addr_w < self.mem_check_point.from_addr)
            && ((addr_w + 1) > self.mem_check_point.to_addr
                || (addr_w + 1) < self.min_addr
                || (addr_w + 1) < self.mem_check_point.from_addr)
        {
            return true;
        }

        self.bus_data_to_input(data);

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
