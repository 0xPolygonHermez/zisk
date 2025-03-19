mod asm_minimal_trace;
mod emu_minimal_trace;

pub use asm_minimal_trace::*;
pub use emu_minimal_trace::*;

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmMinimalTraces),
}
