use std::marker::PhantomData;

use crate::{MemInput, MemModule};
use p3_field::PrimeField;

pub struct MemUnmapped<F: PrimeField> {
    ranges: Vec<(u32, u32)>,
    __data: PhantomData<F>,
}

impl<F: PrimeField> MemUnmapped<F> {
    pub fn new() -> Self {
        Self { ranges: Vec::new(), __data: PhantomData }
    }
    pub fn add_range(&mut self, _start: u32, _end: u32) {
        self.ranges.push((_start, _end));
    }
}
impl<F: PrimeField> MemModule<F> for MemUnmapped<F> {
    fn send_inputs(&self, _mem_op: &[MemInput]) {
        // panic!("[MemUnmapped] invalid access to addr {:x}", _mem_op[0].addr);
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        self.ranges.to_vec()
    }
    fn get_flush_input_size(&self) -> u32 {
        1
    }
}
