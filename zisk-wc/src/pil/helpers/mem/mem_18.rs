extern crate pil2_stark;

use common::{Ptr, TracePol};

pub struct Mem18<T> {
    pub buffer: Option<Vec<u8>>,
    pub ptr: *mut u8,

    pub x: TracePol<T>,
    pub y: TracePol<T>,
}

#[allow(dead_code)]
impl<T> Mem18<T> {
    pub const ROW_SIZE: usize = { std::mem::size_of::<T>() * 2 };
    pub const NUM_ROWS: usize = 1 << 16;

    pub fn new() -> Self {
        let mut buffer = vec![0u8; Self::NUM_ROWS * Self::ROW_SIZE];
        let ptr = buffer.as_mut_ptr();

        let ptr_x = Ptr::new(ptr);

        Self {
            buffer: Some(buffer),
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, Self::NUM_ROWS * Self::ROW_SIZE).as_mut_ptr() },

            x: TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, Self::NUM_ROWS),
            y: TracePol::from_ptr(ptr_x.add::<T>(), Self::ROW_SIZE, Self::NUM_ROWS),
        }
    }

    pub fn from_ptr(ptr: *mut std::ffi::c_void, offset: usize, stride: usize) -> Self {
        let offset = offset * std::mem::size_of::<T>();
        let stride = stride * std::mem::size_of::<T>();

        let mut ptr = ptr as *mut u8;
        ptr = unsafe { ptr.add(offset) };

        let ptr_x = Ptr::new(ptr);

        Self {
            buffer: None,
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, Self::NUM_ROWS * stride).as_mut_ptr() },

            x: TracePol::from_ptr(ptr_x.add::<T>(), stride, Self::NUM_ROWS),
            y: TracePol::from_ptr(ptr_x.add::<T>(), stride, Self::NUM_ROWS),
        }
    }
}
