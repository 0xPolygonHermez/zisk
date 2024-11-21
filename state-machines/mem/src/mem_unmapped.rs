use std::marker::PhantomData;

use crate::MemModule;
use p3_field::PrimeField;

use zisk_core::ZiskRequiredMemory;

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
    fn send_inputs(&self, _mem_op: &[ZiskRequiredMemory]) {
        println!("## MemUnmapped ## access {:?}", _mem_op);
    }
    fn get_addr_ranges(&self) -> Vec<(u32, u32)> {
        self.ranges.to_vec()
    }
    fn get_flush_input_size(&self) -> u64 {
        1024
    }
    fn unregister_predecessor(&self) {}
    fn register_predecessor(&self) {}
}
