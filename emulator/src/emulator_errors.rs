use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ZiskEmulatorErr {
    WrongArguments(ErrWrongArguments),
    AddressOutOfRange(u64),
    EmulationNoCompleted,
    Unknown(String),
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
