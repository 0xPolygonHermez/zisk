pub struct EmuRequiredOperation {
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

pub struct EmuRequiredMemory {
    pub step: u64,
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}

pub struct EmuRequired {
    pub arith: Vec<EmuRequiredOperation>,
    pub binary: Vec<EmuRequiredOperation>,
    pub memory: Vec<EmuRequiredMemory>,
}
