use std::fmt::Display;

pub const DEFAULT_PRINT_VALS: usize = 10;

// TODO: It would be awesome to be able to filter by other field, like the type of operations
//       which is in a column distinct from the opid.
#[derive(Clone)]
pub struct StdMode {
    pub name: ModeName,
    pub opids: Option<Vec<u64>>,
    pub n_vals: usize,
}

impl StdMode {
    pub const fn new(name: ModeName, opids: Option<Vec<u64>>, n_vals: usize) -> Self {
        if n_vals == 0 {
            panic!("n_vals must be greater than 0");
        }

        Self { name, opids, n_vals }
    }
}

impl From<u8> for StdMode {
    fn from(v: u8) -> Self {
        match v {
            0 => StdMode::new(ModeName::Standard, None, DEFAULT_PRINT_VALS),
            1 => StdMode::new(ModeName::Debug, None, DEFAULT_PRINT_VALS),
            _ => panic!("Invalid mode"),
        }
    }
}

impl Default for StdMode {
    fn default() -> Self {
        StdMode::new(ModeName::Standard, None, DEFAULT_PRINT_VALS)
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum ModeName {
    Standard = 0,
    Debug = 1,
}

impl Display for ModeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeName::Standard => write!(f, "Standard"),
            ModeName::Debug => write!(f, "Debug"),
        }
    }
}

impl PartialEq for ModeName {
    fn eq(&self, other: &ModeName) -> bool {
        *self as usize == *other as usize
    }
}

impl PartialEq<ModeName> for usize {
    fn eq(&self, other: &ModeName) -> bool {
        *self == (*other as usize)
    }
}
