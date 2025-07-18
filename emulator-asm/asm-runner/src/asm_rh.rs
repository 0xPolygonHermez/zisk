use std::fmt::Debug;

use crate::{AsmSharedMemory, AsmShmemHeader};

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHHeader {
    pub version: u64,
    pub exit_code: u64,
    pub shmem_allocated_size: u64,
    pub steps: u64,
}

impl AsmShmemHeader for AsmRHHeader {
    fn allocated_size(&self) -> u64 {
        self.shmem_allocated_size
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHData {
    pub steps: u64,
    pub bios_inst_count: Vec<u64>,
    pub prog_inst_count: Vec<u64>,
}

impl AsmRHData {
    pub fn new(steps: u64, bios_inst_count: Vec<u64>, prog_inst_count: Vec<u64>) -> Self {
        AsmRHData { steps, bios_inst_count, prog_inst_count }
    }
}

impl AsmRHData {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub fn from_shared_memory(asm_shared_memory: &AsmSharedMemory<AsmRHHeader>) -> AsmRHData {
        unsafe {
            let data_ptr = asm_shared_memory.data_ptr() as *mut u64;
            // BIOS chunk data
            let bios_data_ptr = data_ptr;
            let bios_len = std::ptr::read(bios_data_ptr) as usize;
            let bios_data_ptr = bios_data_ptr.add(1);
            let bios_inst_count = Vec::from_raw_parts(bios_data_ptr, bios_len, bios_len);

            // Advance pointer after BIOS
            let prog_data_ptr = bios_data_ptr.add(bios_len);

            // Program chunk data
            let prog_len = std::ptr::read(prog_data_ptr) as usize;
            let prog_data_ptr = prog_data_ptr.add(1);
            let prog_inst_count = Vec::from_raw_parts(prog_data_ptr, prog_len, prog_len);

            AsmRHData {
                steps: asm_shared_memory.map_header().steps,
                bios_inst_count,
                prog_inst_count,
            }
        }
    }
}
