use std::{ffi::c_void, fmt::Debug};
use ziskemu::{EmuTrace, EmuTraceStart};

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

impl OutputChunkC {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub unsafe fn to_emu_trace(mapped_ptr: &mut *mut c_void) -> EmuTrace {
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

        let mut registers = [0u64; 32];
        registers[1..].copy_from_slice(&chunk.registers[..31]);

        let mut last_registers = [0u64; 32];
        last_registers[1..].copy_from_slice(&chunk.last_registers[..31]);

        // Return the parsed OutputChunk
        EmuTrace {
            start_state: EmuTraceStart {
                pc: chunk.pc,
                sp: chunk.sp,
                c: chunk.c,
                step: chunk.step,
                regs: registers,
            },
            last_state: EmuTraceStart {
                pc: chunk.last_pc,
                sp: chunk.last_sp,
                c: chunk.last_c,
                step: chunk.last_step,
                regs: last_registers,
            },
            last_mem_reads_index: 0, // TODO! Modify
            end: chunk.end == 1,
            steps: chunk.steps,
            mem_reads,
        }
    }
}
