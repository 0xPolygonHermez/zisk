use std::{ops::{Index, IndexMut}, ptr::null_mut};

#[derive(Debug)]
pub struct TracePol<T: Send + Sync> {
    ptr: *mut u8,
    stride: usize,
    num_rows: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync> TracePol<T> {
    pub fn new(ptr: *mut u8, stride: usize, num_rows: usize) -> Self {
        TracePol {
            ptr,
            stride,
            num_rows,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }
}

impl<T: Send + Sync> Index<usize> for TracePol<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, i: usize) -> &T {
        assert!(i < self.num_rows);
        unsafe { &*(self.ptr.offset((self.stride * i) as isize) as *mut T) }
    }
}

impl<T: Send + Sync> IndexMut<usize> for TracePol<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        assert!(i < self.num_rows);
        unsafe { &mut *(self.ptr.offset((i * self.stride) as isize) as *mut T) }
    }
}

impl<T: Send + Sync> Default for TracePol<T> {
    fn default() -> Self {
        TracePol { ptr: null_mut(), stride: 0, num_rows: 0, _phantom: std::marker::PhantomData }
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
        let trace_pol: TracePol<u8> = TracePol::new(ptr, row_size, num_rows);

        assert_eq!(trace_pol.num_rows(), num_rows);
    }

    #[test]
    fn test_indexing() {
        let ptr = Box::into_raw(Box::new([1u8, 2, 3, 4, 5, 6, 7, 8])) as *mut u8;
        let row_size = std::mem::size_of::<u8>();
        let num_rows = 8;
        let trace_pol: TracePol<u8> = TracePol::new(ptr, row_size, num_rows);

        for i in 0..num_rows {
            assert_eq!(trace_pol[i], i as u8 + 1);
        }
    }

    #[test]
    fn test_index_mut() {
        let ptr = Box::into_raw(Box::new([0u8; 8])) as *mut u8;
        let row_size = std::mem::size_of::<u8>();
        let num_rows = 8;
        let mut trace_pol = TracePol::new(ptr, row_size, num_rows);

        for i in 0..num_rows {
            trace_pol[i] = i as u8 + 1;
        }

        for i in 0..num_rows {
            assert_eq!(trace_pol[i], i as u8 + 1);
        }
    }
}

