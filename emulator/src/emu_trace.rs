//! Emulator trace

use std::fmt::{Debug, Formatter};

/// Trace data at the beginning of the program execution: pc, sp, c and step.
#[repr(C)]
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStart {
    /// Value of the program counter (ROM address)
    pub pc: u64,
    /// Value of the sp register
    pub sp: u64,
    /// Value of the c register
    pub c: u64,
    /// Value of the step
    pub step: u64,
    /// Value of the registers
    pub regs: [u64; 32],
}

/// Trace data of a complete program execution (start, steps, and end) or of a segment of a program
/// execution (also includes last_state).
#[repr(C)]
#[derive(Default, Clone)]
pub struct EmuTrace {
    /// State at the begining of the execution
    pub start_state: EmuTraceStart,

    /// State at the end of the execution
    pub last_state: EmuTraceStart,

    /// Number of steps executed
    pub steps: u64,

    /// Memory reads
    pub mem_reads: Vec<u64>,

    /// Index of the last executed step in memory reads.
    pub last_mem_reads_index: usize,

    /// If the `end` flag is true, the program executed completely.
    /// This does not mean that the program ended successfully; it could have found an error condition
    /// due to, for example, invalid input data, and then jump directly to the end of the ROM.
    /// In this error situation, the output data should reveal the success or fail of the completed
    /// execution.
    /// These are the possible combinations:
    /// * end = false  --> program did not complete, e.g. the emulator run out of steps (you can
    ///   configure more steps)
    /// * end = true --> program completed
    ///   * output data correct --> program completed successfully
    ///   * output data incorrect --> program completed with an error
    pub end: bool,
}

impl Debug for EmuTrace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputChunk")
            .field("pc", &format_args!("{:#x}", self.start_state.pc))
            .field("sp", &format_args!("{:#x}", self.start_state.sp))
            .field("c", &format_args!("{:#x}", self.start_state.c))
            .field("step", &self.start_state.step)
            .field("registers len", &self.start_state.regs.len())
            .field("last_pc", &format_args!("{:#x}", self.last_state.pc))
            .field("last_sp", &format_args!("{:#x}", self.last_state.sp))
            .field("last_c", &format_args!("{:#x}", self.last_state.c))
            .field("last_step", &self.last_state.step)
            .field("last_registers len", &self.last_state.regs.len())
            .field("steps", &format_args!("{:}", self.steps))
            .field("mem reads size", &format_args!("{:}", self.mem_reads.len()))
            .field("last mem reads index", &format_args!("{:}", self.last_mem_reads_index))
            .field("end", &format_args!("{}", self.end))
            .finish()
    }
}
