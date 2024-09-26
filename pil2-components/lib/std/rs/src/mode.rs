use std::fmt::Display;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum StdMode {
    Standard = 0,
    Debug = 1,
}

impl Display for StdMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StdMode::Standard => write!(f, "Standard"),
            StdMode::Debug => write!(f, "Debug"),
        }
    }
}

impl PartialEq for StdMode {
    fn eq(&self, other: &StdMode) -> bool {
        *self as usize == *other as usize
    }
}

impl PartialEq<StdMode> for usize {
    fn eq(&self, other: &StdMode) -> bool {
        *self == (*other as usize)
    }
}
