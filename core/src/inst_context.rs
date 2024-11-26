use crate::{Mem, ROM_ENTRY};

/// ZisK instruction context data container, storing the state of the execution
pub struct InstContext {
    // Memory, including several read-only sections and one read-write section (input data)
    // This memory is initialized before running the program with the input data, and modified by
    // the program instructions during the execution.  The RW data that has not been previously
    // written is read as zero
    pub mem: Mem,

    // Current values of registers a, b, c, and flag
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub flag: bool,

    // Value of sp register
    pub sp: u64,

    // Value of ROM program execution address, i.e. program counter (pc)
    pub pc: u64,

    // Current execution step: 0, 1, 2...
    pub step: u64,

    // End flag, set to true only by the last instruction to execute
    pub end: bool,
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
