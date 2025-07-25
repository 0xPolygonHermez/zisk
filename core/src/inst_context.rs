//! * Provides a context to execute a set of Zisk instructions.
//! * The context contains the state of the Zisk processor, modified by the execution of every
//!   instruction.
//! * The state includes: memory, registers (a, b, c, flag, sp), program counter (pc), step and a
//!   flag to mark the end of the program execution.

use crate::{Mem, REGS_IN_MAIN_TOTAL_NUMBER, ROM_ENTRY};

/// Zisk precompiled
#[derive(Debug, Default, PartialEq, Eq)]
pub enum EmulationMode {
    #[default]
    Mem,
    GenerateMemReads,
    ConsumeMemReads,
}

/// Zisk precompiled instruction context.
/// Stores the input data (of the size expected by the precompiled components) and the output data.
/// If the precompiled component finds input_data not empty, it should use this data instead of
/// reading it from memory
#[derive(Debug, Default)]
pub struct PrecompiledInstContext {
    /// Step
    pub step: u64,

    /// Precompiled input data address
    // pub input_data_address: u64,
    /// Precompiled input data
    pub input_data: Vec<u64>,

    /// Precompiled output data address
    // pub output_data_address: u64,
    /// Precompiled output data
    pub output_data: Vec<u64>,
}

/// Zisk fcall instruction context.
/// Stores the fcall arguments data and the result data.
#[derive(Debug, Default)]
pub struct FcallInstContext {
    /// Fcall parameters data
    /// Maximum size is 32 u64's
    pub parameters: [u64; 32],

    /// Indicates how many parameters u64's contain valid data
    pub parameters_size: u64,

    /// Fcall result data
    /// Maximum size is 32 u64's
    pub result: [u64; 32],

    /// Indicates how many result u64's contain valid data
    pub result_size: u64,

    /// Indicates how many result u64's have been read using fcall_get()
    pub result_got: u64,
}

#[derive(Debug)]
/// ZisK instruction context data container, storing the state of the execution
pub struct InstContext {
    /// Memory, including several read-only sections and one read-write section (input data)
    /// This memory is initialized before running the program with the input data, and modified by
    /// the program instructions during the execution.  The RW data that has not been previously
    /// written is read as zero
    pub mem: Mem,

    /// Current value of register a
    pub a: u64,
    /// Current value of register b
    pub b: u64,
    /// Current value of register c
    pub c: u64,
    /// Current value of register flag
    pub flag: bool,

    /// Current value of register sp
    pub sp: u64,

    /// Current value of ROM program execution address, i.e. program counter (pc)
    pub pc: u64,

    /// Current execution step: 0, 1, 2...
    pub step: u64,

    /// End flag, set to true only by the last instruction to execute
    pub end: bool,

    /// Registers
    pub regs: [u64; REGS_IN_MAIN_TOTAL_NUMBER],

    /// Precompiled emulation mode
    pub emulation_mode: EmulationMode,

    /// Precompiled data
    pub precompiled: PrecompiledInstContext,

    /// Fcall data
    pub fcall: FcallInstContext,
}

/// RisK instruction context implementation
impl InstContext {
    /// RisK instruction context constructor
    pub fn new() -> InstContext {
        InstContext {
            mem: Mem::default(),
            a: 0,
            b: 0,
            c: 0,
            flag: false,
            sp: 0,
            pc: ROM_ENTRY,
            step: 0,
            end: false,
            regs: [0; REGS_IN_MAIN_TOTAL_NUMBER],
            emulation_mode: EmulationMode::default(),
            precompiled: PrecompiledInstContext::default(),
            fcall: FcallInstContext::default(),
        }
    }

    /// Creates a human-readable string describing the instruction context, for debugging purposes
    pub fn to_text(&self) -> String {
        let s = format! {"a={:x} b={:x} c={:x} flag={} sp={} pc={} step={} end={}", self.a, self.b, self.c, self.flag, self.sp, self.pc, self.step, self.end};
        s
    }
}

impl Default for InstContext {
    /// Default instruction context constructor
    fn default() -> Self {
        Self::new()
    }
}
