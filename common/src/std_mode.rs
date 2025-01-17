use std::fmt::Display;

pub const DEFAULT_PRINT_VALS: usize = 10;

// TODO: It would be awesome to be able to filter by other field, like the type of operations
//       which is in a column distinct from the opid.
#[derive(Clone)]
pub struct StdMode {
    pub name: ModeName,
    pub opids: Vec<u64>,
    pub n_vals: usize,
    pub print_to_file: bool,
}

impl StdMode {
    pub const fn new(name: ModeName, opids: Vec<u64>, n_vals: usize, print_to_file: bool) -> Self {
        if name.as_usize() != ModeName::Standard.as_usize() && n_vals == 0 {
            panic!("n_vals must be greater than 0");
        }

        Self { name, opids, n_vals, print_to_file }
    }

    pub fn new_debug() -> Self {
        Self::new(ModeName::Debug, Vec::new(), DEFAULT_PRINT_VALS, false)
    }
}

impl From<u8> for StdMode {
    fn from(v: u8) -> Self {
        match v {
            0 => StdMode::new(ModeName::Standard, Vec::new(), DEFAULT_PRINT_VALS, false),
            1 => StdMode::new(ModeName::Debug, Vec::new(), DEFAULT_PRINT_VALS, false),
            _ => panic!("Invalid mode"),
        }
    }
}

impl Default for StdMode {
    fn default() -> Self {
        StdMode::new(ModeName::Standard, Vec::new(), DEFAULT_PRINT_VALS, false)
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

impl ModeName {
    const fn as_usize(&self) -> usize {
        *self as usize
    }
}
