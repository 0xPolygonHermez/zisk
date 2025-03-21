use crate::{asm_minimal_trace::AsmMinimalTraces, EmuTrace};

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmMinimalTraces),
}
