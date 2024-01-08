use std::ops::{Index, IndexMut};
use std::cell::UnsafeCell;

#[derive(Debug)]
pub struct TracePol<T: Send + Sync> {
    ptr: UnsafeCell<*mut u8>,
    row_size: usize,
    num_rows: usize,
    _phantom: std::marker::PhantomData<T>,
}

unsafe impl<T: Send + Sync> Sync for TracePol<T> {}
unsafe impl<T: Send + Sync> Send for TracePol<T> {}

impl<T: Send + Sync> TracePol<T> {
    pub fn new(ptr: *mut u8, row_size: usize, num_rows: usize) -> Self {
        TracePol {
            ptr: UnsafeCell::new(ptr),
            row_size,
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

    fn index(&self, i: usize) -> &T {
        assert!(i < self.num_rows);
        let ptr = unsafe { *self.ptr.get() };
        unsafe { & *(ptr.offset((i * self.row_size) as isize) as *mut T) }
    }
}

impl<T: Send + Sync> IndexMut<usize> for TracePol<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        assert!(i < self.num_rows);
        let ptr = unsafe { *self.ptr.get() };
        unsafe { &mut *(ptr.offset((i * self.row_size) as isize) as *mut T) }
    }
}

impl<T: Send + Sync> Default for TracePol<T> {
    fn default() -> Self {
        TracePol {
            ptr: UnsafeCell::new(std::ptr::null_mut()),
            row_size: 0,
            num_rows: 0,
            _phantom: std::marker::PhantomData,
        }
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
