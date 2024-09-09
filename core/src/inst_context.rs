use crate::{Mem, ROM_ENTRY};

/// ZisK instruction context data container, storing the state of the execution
pub struct InstContext {
    pub mem: Mem,
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub flag: bool,
    pub sp: u64,
    pub pc: u64,
    pub step: u64,
    pub end: bool,
}

/// RisK instruction context implementation
impl InstContext {
    /// RisK instruction context constructor
    pub fn new() -> InstContext {
        InstContext {
            mem: Mem::new(),
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
}

impl Default for InstContext {
    fn default() -> Self {
        Self::new()
    }
}
