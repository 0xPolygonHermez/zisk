use std::ptr::NonNull;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct TracePol<T> {
    ptr: NonNull<u8>,
    row_size: usize,
    num_rows: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TracePol<T> {
    pub fn new(p_address: *mut u8, row_size: usize, num_rows: usize) -> Self {
        TracePol {
            ptr: unsafe { NonNull::new_unchecked(p_address) },
            row_size,
            num_rows,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Index<usize> for TracePol<T> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        assert!(i < self.num_rows);
        unsafe { &*(self.ptr.as_ptr().offset((i * self.row_size) as isize) as *const T) }
    }
}

impl<T> IndexMut<usize> for TracePol<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        assert!(i < self.num_rows);
        unsafe { &mut *(self.ptr.as_ptr().offset((i * self.row_size) as isize) as *mut T) }
    }
}

pub struct Ptr {
    ptr: NonNull<u8>,
}

impl Ptr {
    pub fn new(p_address: *mut u8) -> Self {
        Ptr {
            ptr: unsafe { NonNull::new_unchecked(p_address) },
        }
    }

    pub fn add<T>(&mut self) -> *mut u8 {
        let old_ptr = self.ptr;
        self.ptr = unsafe { NonNull::new_unchecked(self.ptr.as_ptr().offset(std::mem::size_of::<T>() as isize)) };
        old_ptr.as_ptr()
    }
}