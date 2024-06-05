use std::{
    ops::{Index, IndexMut},
    ptr::null_mut,
};
#[allow(dead_code)]
#[derive(Debug)]
pub struct TracePol<T> {
    buffer: Option<Vec<u8>>,
    ptr: *mut u8,
    stride: usize,
    num_rows: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TracePol<T> {
    pub fn new(num_rows: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        let stride = std::mem::size_of::<T>();
        let mut buffer = vec![0u8; num_rows * stride];

        let ptr = buffer.as_mut_ptr();

        TracePol { buffer: Some(buffer), ptr, stride, num_rows, _phantom: std::marker::PhantomData }
    }

    pub fn from_ptr(ptr: *mut u8, stride: usize, num_rows: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        TracePol { buffer: None, ptr, stride, num_rows, _phantom: std::marker::PhantomData }
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }
}

impl<T> Index<usize> for TracePol<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, i: usize) -> &T {
        assert!(i < self.num_rows);
        unsafe { &*(self.ptr.offset((self.stride * i) as isize) as *mut T) }
    }
}

impl<T> IndexMut<usize> for TracePol<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        assert!(i < self.num_rows);
        unsafe { &mut *(self.ptr.offset((i * self.stride) as isize) as *mut T) }
    }
}

impl<T> Default for TracePol<T> {
    fn default() -> Self {
        TracePol { buffer: None, ptr: null_mut(), stride: 0, num_rows: 0, _phantom: std::marker::PhantomData }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_pol_creation() {
        let ptr = Box::into_raw(Box::new([0u8; 8])) as *mut u8;
        let row_size = std::mem::size_of::<u8>();
        let num_rows = 8;
        let trace_pol: TracePol<u8> = TracePol::from_ptr(ptr, row_size, num_rows);

        assert_eq!(trace_pol.num_rows(), num_rows);
    }

    #[test]
    fn test_indexing() {
        let ptr = Box::into_raw(Box::new([1u8, 2, 3, 4, 5, 6, 7, 8])) as *mut u8;
        let row_size = std::mem::size_of::<u8>();
        let num_rows = 8;
        let trace_pol: TracePol<u8> = TracePol::from_ptr(ptr, row_size, num_rows);

        for i in 0..num_rows {
            assert_eq!(trace_pol[i], i as u8 + 1);
        }
    }

    #[test]
    fn test_index_mut() {
        let ptr = Box::into_raw(Box::new([0u8; 8])) as *mut u8;
        let row_size = std::mem::size_of::<u8>();
        let num_rows = 8;
        let mut trace_pol = TracePol::from_ptr(ptr, row_size, num_rows);

        for i in 0..num_rows {
            trace_pol[i] = i as u8 + 1;
        }

        for i in 0..num_rows {
            assert_eq!(trace_pol[i], i as u8 + 1);
        }
    }
}
