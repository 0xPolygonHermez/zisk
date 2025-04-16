use std::ffi::c_void;

use std::fmt::Debug;

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHHeader {
    pub version: u64,
    pub exit_code: u64,
    pub mt_allocated_size: u64,
    pub steps: u64,
}

impl AsmRHHeader {
    pub fn from_ptr(mapped_ptr: *mut c_void) -> AsmRHHeader {
        let output_header;
        unsafe {
            output_header = std::ptr::read(mapped_ptr as *const AsmRHHeader);
        }

        assert!(output_header.mt_allocated_size > 0);
        assert!(output_header.steps > 0);

        output_header
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHData {
    pub header: AsmRHHeader,
    pub bios_inst_count: Vec<u64>,
    pub prog_inst_count: Vec<u64>,
}

impl AsmRHData {
    pub fn new(header: AsmRHHeader, bios_inst_count: Vec<u64>, prog_inst_count: Vec<u64>) -> Self {
        AsmRHData { header, bios_inst_count, prog_inst_count }
    }
}

impl AsmRHData {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub fn from_ptr(mapped_ptr: &mut *mut c_void, header: AsmRHHeader) -> AsmRHData {
        unsafe {
            // BIOS chunk data
            let bios_data_ptr = *mapped_ptr as *mut u64;
            let bios_len = std::ptr::read(bios_data_ptr) as usize;
            let bios_data_ptr = bios_data_ptr.add(1);
            let bios_inst_count = Vec::from_raw_parts(bios_data_ptr, bios_len, bios_len);

            // Advance pointer after BIOS
            let prog_data_ptr = bios_data_ptr.add(bios_len);

            // Program chunk data
            let prog_len = std::ptr::read(prog_data_ptr) as usize;
            let prog_data_ptr = prog_data_ptr.add(1);
            let prog_inst_count = Vec::from_raw_parts(prog_data_ptr, prog_len, prog_len);

            AsmRHData { header, bios_inst_count, prog_inst_count }
        }
    }
}
