use crate::{EmuTrace, asm_minimal_trace::AsmMinimalTraces};

pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmMinimalTraces),
}
