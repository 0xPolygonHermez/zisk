//use crate::MemTrace;
#[derive(Default)]
pub struct EmuTraceStart {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
}

#[derive(Default)]
pub struct EmuTraceStep {
    pub a: u64,
    pub b: u64,
}
#[derive(Default)]
pub struct EmuTraceEnd {
    pub end: bool,
}

#[derive(Default)]
pub struct EmuTrace {
    pub start: EmuTraceStart,
    pub steps: Vec<EmuTraceStep>,
    pub end: EmuTraceEnd,
}
