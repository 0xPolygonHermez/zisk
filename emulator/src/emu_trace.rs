//use crate::MemTrace;
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStart {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
    pub regs: [u64; 32],
    pub mem_reads_index: usize,
}

#[derive(Default, Debug, Clone)]
pub struct EmuTraceSteps {
    pub mem_reads: Vec<u64>,
    pub steps: u64,
}

#[derive(Default, Debug, Clone)]
pub struct EmuTraceEnd {
    pub end: bool,
}

#[derive(Default, Debug, Clone)]
pub struct EmuTrace {
    pub start_state: EmuTraceStart,
    pub last_state: EmuTraceStart,
    pub steps: EmuTraceSteps,
    pub end: EmuTraceEnd,
}
