use std::fmt::Display;

// TODO: It would be awesome to be able to filter by other field, like the type of operations
//       which is in a column distinct from the opid.
#[derive(Clone)]
pub struct StdMode {
    pub name: ModeName,
    pub opids: Option<Vec<u64>>,
    pub vals_to_print: usize,
}

impl StdMode {
    pub const fn new(name: ModeName, opids: Option<Vec<u64>>, vals_to_print: usize) -> Self {
        if vals_to_print == 0 {
            panic!("vals_to_print must be greater than 0");
        }

        Self { name, opids, vals_to_print }
    }
}

impl Default for StdMode {
    fn default() -> Self {
        StdMode::new(ModeName::Standard, None, 10)
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
