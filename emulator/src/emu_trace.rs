//use crate::MemTrace;
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStart {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
    pub regs: [u64; 32],
}

#[derive(Default, Debug, Clone)]
pub struct EmuTraceStep {
    pub a: u64,
    pub b: u64,
}
#[derive(Default, Debug, Clone)]
pub struct EmuTraceEnd {
    pub end: bool,
}

#[derive(Default, Debug, Clone)]
pub struct EmuTrace {
    pub start_state: EmuTraceStart,
    pub last_state: EmuTraceStart,
    pub steps: Vec<EmuTraceStep>,
    pub end: EmuTraceEnd,
}
