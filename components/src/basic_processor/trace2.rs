use proofman::trace::{trace::Ptr, trace_pol::TracePol};

#[allow(non_snake_case)]
pub struct BasicProcessorTrace2<T> {
    pub buffer: Option<Vec<u8>>,
    pub ptr: *mut u8,
    num_rows: usize,

    pub A: TracePol<[T; 8]>,
}

impl<T> BasicProcessorTrace2<T> {
    const ROW_SIZE: usize = 7 * std::mem::size_of::<[T; 8]>() + 31 * std::mem::size_of::<T>();
    pub fn new(num_rows: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        let mut buffer = vec![0u8; num_rows * Self::ROW_SIZE];

        let ptr = buffer.as_mut_ptr();
        let ptr_x = Ptr::new(ptr);

        BasicProcessorTrace2 {
            buffer: Some(buffer),
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * Self::ROW_SIZE).as_mut_ptr() },
            num_rows,

            A: TracePol::from_ptr(ptr_x.add::<[T; 8]>(), Self::ROW_SIZE, num_rows),
        }
    }

    pub fn from_ptr(ptr: *mut std::ffi::c_void, num_rows: usize, offset: usize, stride: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        let mut ptr = ptr as *mut u8;

        ptr = unsafe { ptr.add(offset) };
        let ptr_x = Ptr::new(ptr);

        BasicProcessorTrace2 {
            buffer: None,
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * stride).as_mut_ptr() },
            num_rows,

            A: TracePol::from_ptr(ptr_x.add::<[T; 8]>(), stride, num_rows),
        }
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn row_size(&self) -> usize {
        Self::ROW_SIZE
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer.as_ref().unwrap().len()
    }
}
