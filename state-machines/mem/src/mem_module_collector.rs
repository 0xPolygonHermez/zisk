use std::collections::VecDeque;

use crate::{MemInput, MemPreviousSegment};
use mem_common::{MemHelpers, MemModuleCheckPoint, MEM_BYTES, MEM_BYTES_BITS};
use zisk_common::{BusDevice, BusId, MemBusData, MemCollectorInfo, SegmentId, MEM_BUS_ID};

#[derive(Debug, PartialEq, Eq)]
enum InputAction {
    Discard,
    Accept,
    SetPrevious, // +Discard
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DualState {
    Ini,
    Read,
    Write,
}

#[derive(Debug)]
pub struct MemModuleCollector {
    /// Binary Basic state machine
    pub mem_check_point: MemModuleCheckPoint,

    /// Collected inputs
    pub inputs: Vec<MemInput>,
    pub prev_segment: Option<MemPreviousSegment>,
    pub min_addr: u32,
    pub filter_min_addr: u32,
    pub filter_max_addr: u32,
    pub aligned_min_addr: u32,
    pub aligned_max_addr: u32,
    pub _segment_id: SegmentId,
    pub count: u32,
    pub to_count: u32,
    pub skip: u32,
    pub is_first_chunk_of_segment: bool,
    pub is_dual: bool,
    state_from: DualState,
    state_to: DualState,
}

impl MemModuleCollector {
    pub fn new(
        mem_check_point: &MemModuleCheckPoint,
        min_addr: u32,
        segment_id: SegmentId,
        is_first_chunk_of_segment: bool,
        is_dual: bool,
    ) -> Self {
        // let prev_addr = mem_check_point.reference_addr;
        // let prev_step = MemHelpers::first_chunk_mem_step(mem_check_point.reference_addr_chunk);
        let count = mem_check_point.count;
        let to_count = mem_check_point.to_count;
        let skip = mem_check_point.from_skip;
        let aligned_min_addr = std::cmp::max(min_addr, mem_check_point.from_addr);
        let aligned_max_addr = mem_check_point.to_addr;
        Self {
            // rows could be dual, dual inputs by row
            inputs: Vec::with_capacity(count as usize * 2),
            mem_check_point: mem_check_point.clone(),
            prev_segment: None,
            min_addr,
            aligned_min_addr,
            aligned_max_addr,
            filter_min_addr: aligned_min_addr << MEM_BYTES_BITS,
            filter_max_addr: (aligned_max_addr << MEM_BYTES_BITS) + MEM_BYTES - 1,
            _segment_id: segment_id,
            count,
            to_count,
            skip,
            is_first_chunk_of_segment,
            state_from: DualState::Ini,
            state_to: DualState::Ini,
            is_dual,
        }
    }

    fn update_state(state: DualState, is_write: bool) -> DualState {
        if is_write {
            DualState::Write
        } else {
            match state {
                DualState::Ini => DualState::Read,
                DualState::Read | DualState::Write => DualState::Ini,
            }
        }
    }

    fn discard_align_addr(&mut self, addr_w: u32) -> bool {
        // Check if the address is out of the range of the current checkpoint, or
        // out of memory area.
        addr_w < self.aligned_min_addr || addr_w > self.filter_max_addr
    }

    #[inline(always)]
    fn action_addr(&mut self, addr_w: u32, is_write: bool) -> InputAction {
        if self.is_dual {
            self.dual_action_addr(addr_w, is_write)
        } else {
            self.non_dual_action_addr(addr_w)
        }
    }

    /// Determines the action to take for a memory access based on address and operation type.
    ///
    /// This function analyzes a memory access to determine whether it should be accepted for processing,
    /// discarded, or stored as previous segment data. The decision is based on address range checks,
    /// skip counters.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address aligned to 8 bytes.
    ///
    /// # Returns
    /// - `InputAction::Discard`: The access should be ignored
    /// - `InputAction::Accept`: The access should be processed and added to inputs
    /// - `InputAction::SetPrevious`: The access should be stored as previous segment data
    ///
    /// # IMPORTANT
    /// - after each non_dual_action_addr call, we need to call process_addr_action
    /// - assumes called previously the discard_align_addr to check if is out of range
    fn non_dual_action_addr(&mut self, addr_w: u32) -> InputAction {
        if addr_w == self.mem_check_point.from_addr && self.skip > 0 {
            self.skip -= 1;
            if self.skip == 0 && self.is_first_chunk_of_segment {
                // The last discart before accept, we need to store the previous segment data
                return InputAction::SetPrevious;
            }

            return InputAction::Discard;
        }

        if addr_w == self.mem_check_point.to_addr && self.to_count == 0 {
            return InputAction::Discard;
        }

        // from_addr <= addr <= to_addr
        if addr_w == self.mem_check_point.to_addr {
            self.to_count -= 1;
        }
        InputAction::Accept
    }
    /// Determines the action to take for a memory access based on address and operation type.
    ///
    /// This function analyzes a memory access to determine whether it should be accepted for processing,
    /// discarded, or stored as previous segment data. The decision is based on address range checks,
    /// skip counters, and state machine transitions that track read/write patterns.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address aligned to 8 bytes.
    /// - `is_write`: Whether this is a write operation (true) or read operation (false).
    ///
    /// # Returns
    /// - `InputAction::Discard`: The access should be ignored
    /// - `InputAction::Accept`: The access should be processed and added to inputs
    /// - `InputAction::SetPrevious`: The access should be stored as previous segment data
    ///
    /// # IMPORTANT
    /// - after each dual_action_addr call, we need to call process_addr_action
    /// - assumes called previously the discard_align_addr to check if is out of range
    fn dual_action_addr(&mut self, addr_w: u32, is_write: bool) -> InputAction {
        if addr_w == self.mem_check_point.from_addr && self.skip > 0 {
            // skip > 1 && !is_first_chunk
            //     ST_INI + X => ST_X, Discard
            //     ST_X + R => ST_INI, skip -= 1, Discard
            //     ST_X + W => ST_W, skip -= 1, Discard
            //
            // skip > 1 && is_first_chunk
            //     ST_INI + X => ST_X, Discard
            //     ST_X + R => ST_INI, skip -= 1, Discard
            //     ST_X + W => ST_W, skip -= 1, if skip == 1 {SetPrevious} else {Discard}
            //
            // skip == 1 && !is_first_chunk
            //     ST_INI + X => ST_X, Discard
            //     ST_X + R => ST_INI, skip -= 1, Discard
            //     ST_X + W => ST_W, skip -= 1, Accept
            //
            // skip == 1 && is_first_chunk
            //     ST_INI + X => ST_X, SetPrevious (Discard)
            //     ST_X + R => ST_INI, skip -= 1, SetPrevious  (Discard)
            //     ST_X + W => ST_W, skip -= 1, (*) Accept
            // (*) previously return SetPrevious

            let prev_state = self.state_from;
            self.state_from = Self::update_state(prev_state, is_write);

            if prev_state == DualState::Ini {
                if self.skip == 1 && self.is_first_chunk_of_segment {
                    return InputAction::SetPrevious;
                } else {
                    return InputAction::Discard;
                }
            }
            self.skip -= 1;

            if self.state_from != DualState::Write {
                if self.skip == 0 && self.is_first_chunk_of_segment {
                    // The last discart before accept, we need to store the previous segment data
                    return InputAction::SetPrevious;
                }
                return InputAction::Discard;
            }

            if self.is_first_chunk_of_segment {
                if self.skip == 0 {
                    return InputAction::Accept;
                } else if self.skip == 1 {
                    return InputAction::SetPrevious;
                } else {
                    return InputAction::Discard;
                }
            } else if self.skip == 0 {
                return InputAction::Accept;
            } else {
                return InputAction::Discard;
            }
        }

        if addr_w == self.mem_check_point.to_addr && self.to_count == 0 {
            return InputAction::Discard;
        }

        if addr_w == self.mem_check_point.to_addr {
            //  ST_INI + X => ST_X, Accept
            //  ST_X + R => ST_INI, count -= 1, Accept
            //  ST_X + W => ST_W, count -= 1, if count == 0 { Discard } else { Accept }

            let prev_state = self.state_to;
            self.state_to = Self::update_state(prev_state, is_write);

            if prev_state == DualState::Ini {
                return InputAction::Accept;
            }

            self.to_count -= 1;

            if self.state_to == DualState::Write && self.to_count == 0 {
                return InputAction::Discard;
            }
        }

        // from_addr <= addr <= to_addr
        InputAction::Accept
    }

    /// Processes an unaligned memory access by determining the required aligned operations.
    ///
    /// This method handles unaligned memory accesses by computing all necessary aligned memory
    /// operations required to validate the unaligned access. It determines the specific access
    /// case based on whether it involves a single or double memory access and delegates to the
    /// appropriate specialized method.
    ///
    /// # Parameters
    /// - `addr`: The unaligned memory address.
    /// - `bytes`: The number of bytes in the memory access.
    /// - `is_write`: Whether this is a write operation (true) or read operation (false).
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_data(&mut self, addr: u32, bytes: u8, data: &[u64]) {
        let addr_w = MemHelpers::get_addr_w(addr);
        if MemHelpers::is_double(addr, bytes) {
            let discard_addr_1 = self.discard_align_addr(addr_w);
            let discard_addr_2 = self.discard_align_addr(addr_w + 1);
            if discard_addr_1 && discard_addr_2 {
                return;
            }

            // In this point exits the possibilit that one of the two
            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            if is_write {
                self.process_unaligned_double_write(
                    addr_w,
                    bytes,
                    data,
                    discard_addr_1,
                    discard_addr_2,
                );
            } else {
                self.process_unaligned_double_read(addr_w, data, discard_addr_1, discard_addr_2);
            }
        } else {
            if self.discard_align_addr(addr_w) {
                return;
            }
            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            if is_write {
                self.process_unaligned_single_write(addr_w, bytes, data);
            } else {
                self.process_unaligned_single_read(addr_w, data);
            }
        }
    }

    /// Processes an unaligned single read operation.
    ///
    /// Handles an unaligned single read operation by computing all necessary aligned memory
    /// operations required to validate the unaligned access. Finally, it uses `process_addr_action`
    /// to manage the action required for the unaligned access.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (aligned to 8 bytes).
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_single_read(&mut self, addr_w: u32, data: &[u64]) {
        let action = self.action_addr(addr_w, false);
        if action == InputAction::Discard {
            return;
        }
        let value = MemBusData::get_mem_values(data)[0];
        let step = MemBusData::get_step(data);
        self.process_addr_action(addr_w, step, value, false, action);
    }

    /// Processes an unaligned single write operation.
    ///
    /// Handles an unaligned single write operation by computing all necessary aligned memory
    /// operations required to validate the unaligned access. Additionally, it calculates the
    /// write value for the given memory access. Finally, it uses `process_addr_action` to
    /// manage the action required for the unaligned access.
    ///
    /// # Parameters
    /// - `addr_w`: The memory address (aligned to 8 bytes).
    /// - `bytes`: The number of bytes to be written.
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_single_write(&mut self, addr_w: u32, bytes: u8, data: &[u64]) {
        let action_read = self.action_addr(addr_w, false);
        let step = MemBusData::get_step(data);
        let read_values = MemBusData::get_mem_values(data);
        let read_step = MemHelpers::get_read_step(step);
        self.process_addr_action(addr_w, read_step, read_values[0], false, action_read);

        let action_write = self.action_addr(addr_w, true);
        if action_write != InputAction::Discard {
            let write_step = MemHelpers::get_write_step(step);
            let write_values = MemHelpers::get_write_values(
                MemBusData::get_addr(data),
                bytes,
                MemBusData::get_value(data),
                read_values,
            );
            self.process_addr_action(addr_w, write_step, write_values[0], true, action_write);
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
    fn process_unaligned_double_read(
        &mut self,
        addr_w: u32,
        data: &[u64],
        discard_addr_1: bool,
        discard_addr_2: bool,
    ) {
        let read_values = MemBusData::get_mem_values(data);
        let step = MemBusData::get_step(data);
        if !discard_addr_1 {
            let action = self.action_addr(addr_w, false);
            self.process_addr_action(addr_w, step, read_values[0], false, action);
        }
        if !discard_addr_2 {
            let action = self.action_addr(addr_w + 1, false);
            self.process_addr_action(addr_w + 1, step, read_values[1], false, action);
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
    fn process_unaligned_double_write(
        &mut self,
        addr_w: u32,
        bytes: u8,
        data: &[u64],
        discard_addr_1: bool,
        discard_addr_2: bool,
    ) {
        // IMPORTANT: inputs must be ordered by step
        let read_values = MemBusData::get_mem_values(data);
        let step = MemBusData::get_step(data);
        let read_step = MemHelpers::get_read_step(step);

        if !discard_addr_1 {
            let action = self.action_addr(addr_w, false);
            self.process_addr_action(addr_w, read_step, read_values[0], false, action);
        }
        if !discard_addr_2 {
            let action = self.action_addr(addr_w + 1, false);
            self.process_addr_action(addr_w + 1, read_step, read_values[1], false, action);
        }

        let write_step = MemHelpers::get_write_step(step);
        let write_values = MemHelpers::get_write_values(
            MemBusData::get_addr(data),
            bytes,
            MemBusData::get_value(data),
            read_values,
        );
        if !discard_addr_1 {
            let action = self.action_addr(addr_w, true);
            self.process_addr_action(addr_w, write_step, write_values[0], true, action);
        }
        if !discard_addr_2 {
            let action = self.action_addr(addr_w + 1, true);
            self.process_addr_action(addr_w + 1, write_step, write_values[1], true, action);
        }
    }

    #[inline(always)]
    fn process_addr_action(
        &mut self,
        addr_w: u32,
        step: u64,
        value: u64,
        is_write: bool,
        action: InputAction,
    ) {
        match action {
            InputAction::Discard => {}
            InputAction::Accept => {
                self.inputs.push(MemInput::new(addr_w, is_write, step, value));
            }
            InputAction::SetPrevious => {
                self.prev_segment = Some(MemPreviousSegment { addr: addr_w, step, value });
            }
        }
    }
    fn bus_data_to_input(&mut self, addr: u32, data: &[u64]) {
        // decoding information in bus

        let bytes = MemBusData::get_bytes(data);
        if MemHelpers::is_aligned(addr, bytes) {
            // Direct case when is aligned, calculated 8 bytes addres (addr_w) and check if is a
            // write or read.

            let addr_w = MemHelpers::get_addr_w(addr);
            if self.discard_align_addr(addr_w) {
                return;
            }

            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            let action = self.action_addr(addr_w, is_write);
            if action != InputAction::Discard {
                let step = MemBusData::get_step(data);
                if is_write {
                    let value = MemBusData::get_value(data);
                    self.process_addr_action(addr_w, step, value, true, action);
                } else {
                    let value = MemBusData::get_mem_values(data)[0];
                    self.process_addr_action(addr_w, step, value, false, action);
                }
            }
        } else {
            // If the access is unaligned (not aligned to 8 bytes or has a width different from 8 bytes).
            self.process_unaligned_data(addr, bytes, data);
        }
    }

    pub fn get_mem_collector_info(&self) -> MemCollectorInfo {
        MemCollectorInfo { from_addr: self.filter_min_addr, to_addr: self.filter_max_addr }
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

        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);
        if (addr + bytes as u32) > self.filter_min_addr && addr <= self.filter_max_addr {
            self.bus_data_to_input(addr, data);
        }
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
