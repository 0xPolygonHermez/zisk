#[derive(Clone)]
pub struct ZiskRequiredOperation {
    pub step: u64,
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

pub struct ZiskRequiredMemory {
    pub step: u64,
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}

#[derive(Clone, Default)]
pub struct ZiskRequiredBinaryBasicTable {
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
    pub row: u64,
    pub multiplicity: u64,
}

#[derive(Clone, Default)]
pub struct ZiskRequiredBinaryExtensionTable {
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
    pub offset: u64,
    pub row: u64,
    pub multiplicity: u64,
}

#[derive(Clone, Default)]
pub struct ZiskRequiredRangeCheck {
    pub rc: u64,
}

pub struct ZiskRequired {
    pub arith: Vec<ZiskRequiredOperation>,
    pub binary: Vec<ZiskRequiredOperation>,
    pub memory: Vec<ZiskRequiredMemory>,
}
