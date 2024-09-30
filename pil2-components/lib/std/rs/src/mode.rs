use std::fmt::Display;

// TODO: It would be awesome to be able to filter by other field, like the type of operations
//       which is in a column distinct from the opid.
#[derive(Clone)]
pub struct StdMode {
    pub name: ModeName,
    pub opids: Option<Vec<u64>>,
}

impl StdMode {
    pub const fn new(name: ModeName) -> Self {
        Self { name, opids: None }
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
