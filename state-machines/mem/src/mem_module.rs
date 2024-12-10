use crate::{MemHelpers, MemInput, MEM_BYTES};
use zisk_core::ZiskRequiredMemory;

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64, is_internal: bool) -> Self {
        MemInput { addr, is_write, step, value, is_internal }
    }
    pub fn from(mem_op: &ZiskRequiredMemory) -> Self {
        match mem_op {
            ZiskRequiredMemory::Basic { step, value, address, is_write, width, step_offset } => {
                debug_assert_eq!(*width, MEM_BYTES as u8);
                MemInput {
                    addr: address >> 3,
                    is_write: *is_write,
                    is_internal: false,
                    step: MemHelpers::main_step_to_address_step(*step, *step_offset),
                    value: *value,
                }
            }
            ZiskRequiredMemory::Extended { values: _, address: _ } => {
                panic!("MemInput::from() called with an extended instance");
            }
        }
    }
}

pub trait MemModule<F>: Send + Sync {
    fn send_inputs(&self, mem_op: &[MemInput]);
    fn get_addr_ranges(&self) -> Vec<(u32, u32)>;
    fn get_flush_input_size(&self) -> u32;
}
