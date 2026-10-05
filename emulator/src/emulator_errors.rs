use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ZiskEmulatorErr {
    WrongArguments(ErrWrongArguments),
    AddressOutOfRange(u64),
    EmulationNoCompleted,
    /// The guest execution failed, for `reason`, at the instruction at this step and pc. A
    /// failed execution must never be proven.
    ExecutionFailed {
        reason: FailureReason,
        step: u64,
        pc: u64,
    },
    Unknown(String),
}

/// Why a guest execution failed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
    /// An illegal instruction: a trap (unimp, ebreak), a write to a read-only CSR, ...
    Trap,
    /// The guest exited with this nonzero exit code (the a0 of its exit call)
    ExitCode(u64),
}

#[derive(Debug)]
pub struct ErrWrongArguments {
    pub description: String,
}

impl ErrWrongArguments {
    // Accept any type that can be converted into a String
    pub fn new<D>(description: D) -> ErrWrongArguments
    where
        D: Into<String>,
    {
        ErrWrongArguments { description: description.into() }
    }
}

impl fmt::Display for ZiskEmulatorErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZiskEmulatorErr::WrongArguments(e) => write!(f, "{e}"),
            ZiskEmulatorErr::AddressOutOfRange(addr) => {
                write!(f, "Address out of range: {addr:#x}")
            }
            ZiskEmulatorErr::EmulationNoCompleted => write!(f, "Emulation not completed"),
            ZiskEmulatorErr::ExecutionFailed { reason: FailureReason::Trap, step, pc } => {
                write!(f, "Guest execution failed (trap) at step={step} pc={pc:#x}")
            }
            ZiskEmulatorErr::ExecutionFailed {
                reason: FailureReason::ExitCode(code),
                step,
                pc,
            } => {
                write!(f, "Guest exited with code {} at step={step} pc={pc:#x}", *code as i64)
            }
            ZiskEmulatorErr::Unknown(code) => write!(f, "Error code {code}"),
        }
    }
}

impl Error for ZiskEmulatorErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ZiskEmulatorErr::WrongArguments(e) => Some(e),
            ZiskEmulatorErr::AddressOutOfRange(_) => None,
            ZiskEmulatorErr::EmulationNoCompleted => None,
            ZiskEmulatorErr::ExecutionFailed { .. } => None,
            ZiskEmulatorErr::Unknown(_) => None,
        }
    }
}

impl fmt::Display for ErrWrongArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.description)
    }
}

impl Error for ErrWrongArguments {}
