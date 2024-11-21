use std::collections::HashMap;
use std::fmt;

#[derive(Clone)]
pub struct ZiskRequiredOperation {
    pub step: u64,
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

#[derive(Clone)]
pub struct ZiskRequiredMemory {
    pub address: u32,
    pub is_write: bool,
    pub width: u8,
    pub step: u64,
    pub value: u64,
}

impl fmt::Debug for ZiskRequiredMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = if self.is_write { "WR" } else { "RD" };
        write!(
            f,
            "{0} addr:{1:#08X}({1}) with:{2} value:{3:#016X}({3}) step:{4} offset:{5}",
            label,
            self.address,
            self.width,
            self.value,
            self.step,
            self.address & 0x07
        )
    }
}

#[derive(Clone, Default)]
pub struct ZiskRequired {
    pub arith: Vec<ZiskRequiredOperation>,
    pub binary: Vec<ZiskRequiredOperation>,
    pub binary_extension: Vec<ZiskRequiredOperation>,
    pub memory: Vec<ZiskRequiredMemory>,
}

#[derive(Clone, Default)]
pub struct ZiskPcHistogram {
    pub map: HashMap<u64, u64>,
    pub end_pc: u64,
    pub steps: u64,
}
