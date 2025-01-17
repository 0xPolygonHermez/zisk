pub mod cli;
pub mod timer_macro;

use p3_field::Field;
use std::mem::MaybeUninit;

pub fn create_buffer_fast<F: Field>(buffer_size: usize) -> Vec<F> {
    let mut buffer: Vec<MaybeUninit<F>> = Vec::with_capacity(buffer_size);
    unsafe {
        buffer.set_len(buffer_size);
    }
    let buffer: Vec<F> = unsafe { std::mem::transmute(buffer) };
    buffer
}

pub fn create_buffer_fast_u8(buffer_size: usize) -> Vec<u8> {
    let mut buffer: Vec<MaybeUninit<u8>> = Vec::with_capacity(buffer_size);
    unsafe {
        buffer.set_len(buffer_size);
    }
    let buffer: Vec<u8> = unsafe { std::mem::transmute(buffer) };
    buffer
}
