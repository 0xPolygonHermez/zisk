use crate::{EmuTrace, asm_minimal_trace::AsmMinimalTraces};

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmMinimalTraces),
}
