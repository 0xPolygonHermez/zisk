use std::collections::HashMap;

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
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        s += &format! {" address={} = {:x}", self.address, self.address};
        s += &(" step=".to_string() + &self.step.to_string());
        s += &(" value=".to_string() + &self.value.to_string());
        s += &(" is_write=".to_string() + &self.is_write.to_string());
        s += &(" width=".to_string() + &self.width.to_string());
        s
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
