use std::{
    ffi::c_void,
    fmt::{Debug, Formatter},
};

#[repr(C)]
#[derive(Debug)]
pub struct OutputHeader {
    pub version: u64,
    pub exit_code: u64,
    pub mt_allocated_size: u64,
    pub mt_used_size: u64,
}

impl OutputHeader {
    pub fn from_ptr(mapped_ptr: *mut c_void) -> OutputHeader {
        let output_header;
        unsafe {
            output_header = std::ptr::read(mapped_ptr as *const OutputHeader);
        }

        assert!(output_header.mt_allocated_size > 0);
        assert!(output_header.mt_used_size > 0);

        output_header
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct OutputChunkC {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
    pub registers: [u64; 33],
    pub last_pc: u64,
    pub last_sp: u64,
    pub last_c: u64,
    pub last_step: u64,
    pub last_registers: [u64; 33],
    pub end: u64,
    pub steps: u64,
    pub mem_reads_size: u64,
}

#[repr(C)]
pub struct OutputChunk {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
    pub registers: [u64; 33],

    pub last_pc: u64,
    pub last_sp: u64,
    pub last_c: u64,
    pub last_step: u64,
    pub last_registers: [u64; 33],

    pub end: u64,
    pub steps: u64,

    pub mem_reads: Vec<u64>,
}

impl From<OutputChunkC> for OutputChunk {
    fn from(chunk: OutputChunkC) -> Self {
        Self {
            pc: chunk.pc,
            sp: chunk.sp,
            c: chunk.c,
            step: chunk.step,
            registers: chunk.registers,
            last_pc: chunk.last_pc,
            last_sp: chunk.last_sp,
            last_c: chunk.last_c,
            last_step: chunk.last_step,
            last_registers: chunk.last_registers,
            end: chunk.end,
            steps: chunk.steps,
            mem_reads: Vec::new(),
        }
    }
}

impl Debug for OutputChunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputChunk")
            .field("pc", &format_args!("{:#x}", self.pc))
            .field("sp", &format_args!("{:#x}", self.sp))
            .field("c", &format_args!("{:#x}", self.c))
            .field("step", &self.step)
            .field("registers", &self.registers)
            .field("last_pc", &format_args!("{:#x}", self.last_pc))
            .field("last_sp", &format_args!("{:#x}", self.last_sp))
            .field("last_c", &format_args!("{:#x}", self.last_c))
            .field("last_step", &self.last_step)
            .field("last_registers", &self.last_registers)
            .field("end", &format_args!("{:#x}", self.end))
            .field("steps", &format_args!("{:}", self.steps))
            .field("mem reads size", &format_args!("{:}", self.mem_reads.len()))
            .finish()
    }
}

impl OutputChunk {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub unsafe fn from_ptr(mapped_ptr: &mut *mut c_void) -> OutputChunk {
        // Read chunk data
        let chunk = std::ptr::read(*mapped_ptr as *const OutputChunkC);
        *mapped_ptr =
            (*mapped_ptr as *mut u8).add(std::mem::size_of::<OutputChunkC>()) as *mut c_void;

        // Convert mem_reads into a Vec<u64> without copying
        let mem_reads_ptr = *mapped_ptr as *mut u64;
        let mem_reads_len = chunk.mem_reads_size as usize;
        let mem_reads = Vec::from_raw_parts(mem_reads_ptr, mem_reads_len, mem_reads_len);

        // Advance the pointer after reading memory reads
        *mapped_ptr = (*mapped_ptr as *mut u64).add(mem_reads_len) as *mut c_void;

        // Return the parsed OutputChunk
        OutputChunk {
            pc: chunk.pc,
            sp: chunk.sp,
            c: chunk.c,
            step: chunk.step,
            registers: chunk.registers,
            last_pc: chunk.last_pc,
            last_sp: chunk.last_sp,
            last_c: chunk.last_c,
            last_step: chunk.last_step,
            last_registers: chunk.last_registers,
            end: chunk.end,
            steps: chunk.steps,
            mem_reads,
        }
    }
}
