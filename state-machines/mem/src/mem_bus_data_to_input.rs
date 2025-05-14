use crate::{MemHelpers, MemInput};
use zisk_common::MemBusData;

pub struct MemBusDataToInput;

impl MemBusDataToInput {
    /// Processes an unaligned memory access.
    ///
    /// Processes an unaligned memory access by computing all necessary aligned memory operations
    /// required to validate the unaligned access. The method determines the specific access case
    /// based on whether it involves a single or double memory access and calls the appropriate
    /// method to handle it.
    ///
    /// # Parameters
    /// - `data`: The data associated with the memory access.
    fn process_unaligned_data(data: &[u64]) -> Vec<MemInput> {
        let addr = MemBusData::get_addr(data);
        let addr_w = MemHelpers::get_addr_w(addr);
        let bytes = MemBusData::get_bytes(data);
        let is_write = MemHelpers::is_write(MemBusData::get_op(data));
        if MemHelpers::is_double(addr, bytes) {
            if is_write {
                Self::process_unaligned_double_write(addr_w, bytes, data)
            } else {
                Self::process_unaligned_double_read(addr_w, data)
            }
        } else if is_write {
            Self::process_unaligned_single_write(addr_w, bytes, data)
        } else {
            Self::process_unaligned_single_read(addr_w, data)
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
    fn process_unaligned_single_read(addr_w: u32, data: &[u64]) -> Vec<MemInput> {
        let value = MemBusData::get_mem_values(data)[0];
        let step = MemBusData::get_step(data);
        vec![MemInput::new(addr_w, false, step, value)]
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
    fn process_unaligned_single_write(addr_w: u32, bytes: u8, data: &[u64]) -> Vec<MemInput> {
        let read_values = MemBusData::get_mem_values(data);
        let write_values = MemHelpers::get_write_values(
            MemBusData::get_addr(data),
            bytes,
            MemBusData::get_value(data),
            read_values,
        );
        let step = MemBusData::get_step(data);
        vec![
            MemInput::new(addr_w, false, MemHelpers::get_read_step(step), read_values[0]),
            MemInput::new(addr_w, true, MemHelpers::get_write_step(step), write_values[0]),
        ]
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
    fn process_unaligned_double_read(addr_w: u32, data: &[u64]) -> Vec<MemInput> {
        let read_values = MemBusData::get_mem_values(data);
        let step = MemBusData::get_step(data);
        vec![
            MemInput::new(addr_w, false, step, read_values[0]),
            MemInput::new(addr_w + 1, false, step, read_values[1]),
        ]
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
    fn process_unaligned_double_write(addr_w: u32, bytes: u8, data: &[u64]) -> Vec<MemInput> {
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
        vec![
            MemInput::new(addr_w, false, read_step, read_values[0]),
            MemInput::new(addr_w + 1, false, read_step, read_values[1]),
            MemInput::new(addr_w, true, write_step, write_values[0]),
            MemInput::new(addr_w + 1, true, write_step, write_values[1]),
        ]
    }

    pub fn bus_data_to_input(data: &[u64]) -> Vec<MemInput> {
        // decoding information in bus

        let addr = MemBusData::get_addr(data);
        let bytes = MemBusData::get_bytes(data);

        // If the access is unaligned (not aligned to 8 bytes or has a width different from 8 bytes).

        if !MemHelpers::is_aligned(addr, bytes) {
            Self::process_unaligned_data(data)
        } else {
            // Direct case when is aligned, calculated 8 bytes addres (addr_w) and check if is a
            // write or read.

            let step = MemBusData::get_step(data);
            let addr_w = MemHelpers::get_addr_w(addr);
            let is_write = MemHelpers::is_write(MemBusData::get_op(data));
            if is_write {
                vec![MemInput::new(addr_w, true, step, MemBusData::get_value(data))]
            } else {
                vec![MemInput::new(addr_w, false, step, MemBusData::get_mem_values(data)[0])]
            }
        }
    }
}
