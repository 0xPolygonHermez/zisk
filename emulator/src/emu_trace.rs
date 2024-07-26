use crate::MemTrace;

pub struct EmuTrace {
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub flag: bool,
    pub sp: u64,
    pub pc: u64,
    pub step: u64,
    pub end: bool,
    pub mem_trace: Vec<MemTrace>,
}

/// Default constructor
impl Default for EmuTrace {
    fn default() -> Self {
        Self {
            opcode: 0,
            a: 0,
            b: 0,
            c: 0,
            flag: false,
            sp: 0,
            pc: 0,
            step: 0,
            end: false,
            mem_trace: Vec::new(),
        }
    }
}
