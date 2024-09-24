use log::LevelFilter;

pub enum VerboseMode {
    Info,
    Debug,
    Trace,
}

impl VerboseMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Info,
            1 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

impl From<VerboseMode> for LevelFilter {
    fn from(val: VerboseMode) -> Self {
        match val {
            VerboseMode::Info => LevelFilter::Info,
            VerboseMode::Debug => LevelFilter::Debug,
            VerboseMode::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    Disabled,
    Error,
    XXX,
    Trace,
}

impl DebugMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Disabled,
            1 => Self::Error,
            2 => Self::XXX,
            3 => Self::Trace,
            _ => Self::Disabled,
        }
    }
}
