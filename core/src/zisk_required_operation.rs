#[derive(Clone)]
pub struct ZiskRequiredOperation {
    pub step: u64,
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

#[derive(Clone)]
pub struct ZiskRequiredMemory {
    pub step: u64,
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}

impl ZiskRequiredMemory {
    pub fn new(step: u64, is_write: bool, address: u64, width: u64, value: u64) -> Self {
        Self {
            step,
            is_write,
            address,
            width,
            value,
        }
    }
}

#[derive(Clone, Default)]
pub struct ZiskRequiredBinaryBasicTable {
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
    pub cin: u64,
    pub last: u64,
}

#[derive(Clone, Default)]
pub struct ZiskRequiredBinaryExtensionTable {
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
    pub offset: u64,
}

pub struct ZiskRequired {
    pub arith: Vec<ZiskRequiredOperation>,
    pub binary: Vec<ZiskRequiredOperation>,
    pub memory: Vec<ZiskRequiredMemory>,
}
