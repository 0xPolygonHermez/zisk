use zisk_common::EmuTrace;

pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    // AsmEmuTrace(AsmMinimalTraces),
}