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

#[derive(Clone, Default)]
pub struct ZiskRequired {
    pub arith: Vec<ZiskRequiredOperation>,
    pub binary: Vec<ZiskRequiredOperation>,
    pub binary_extension: Vec<ZiskRequiredOperation>,
    pub memory: Vec<ZiskRequiredMemory>,
}
