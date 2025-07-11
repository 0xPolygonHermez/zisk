//! Emulator trace

use std::fmt::{Debug, Formatter};

use zisk_core::REGS_IN_MAIN_TOTAL_NUMBER;

/// Trace data at the beginning of the program execution: pc, sp, c and step.
#[repr(C)]
#[derive(Debug, Clone)]
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
    pub regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],
}

impl Default for EmuTraceStart {
    fn default() -> EmuTraceStart {
        EmuTraceStart { pc: 0, sp: 0, c: 0, step: 0, regs: [0; REGS_IN_MAIN_TOTAL_NUMBER] }
    }
}

/// Trace data of a complete program execution (start, steps, and end) or of a segment of a program
/// execution (also includes last_state).
#[repr(C)]
#[derive(Default, Clone)]
pub struct EmuTrace {
    /// State at the beginning of the execution
    pub start_state: EmuTraceStart,
    /// State at the end of the execution
    pub last_c: u64,
    /// Number of steps executed
    pub steps: u64,
    /// Memory reads
    pub mem_reads: Vec<u64>,

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
            .field("last_c", &format_args!("{:#x}", self.last_c))
            .field("steps", &format_args!("{:}", self.steps))
            .field("mem reads size", &format_args!("{:}", self.mem_reads.len()))
            .field("end", &format_args!("{}", self.end))
            .finish()
    }
}
