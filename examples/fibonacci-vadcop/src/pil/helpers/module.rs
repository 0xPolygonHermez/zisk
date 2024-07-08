use wcmanager::{TracePol, Ptr};

#[allow(dead_code)]
pub struct ModuleTrace<F> {
    pub x: TracePol<F>,
    pub q: TracePol<F>,
    pub x_mod: TracePol<F>,
    num_rows: usize,
    pub buffer: Option<Vec<u8>>,
    pub ptr: *mut u8,
}

#[allow(dead_code)]
impl<F> ModuleTrace<F> {
    const ROW_SIZE: usize = 2 * std::mem::size_of::<F>();

    pub fn new(num_rows: usize) -> Self {
        // PRECONDITIONS
        // num_rows must be greater than or equal to 2
        assert!(num_rows >= 2);
        // num_rows must be a power of 2
        assert!(num_rows & (num_rows - 1) == 0);

        let mut buffer = vec![0u8; num_rows * Self::ROW_SIZE];

        let ptr = buffer.as_mut_ptr();
        let ptr_x = Ptr::new(ptr);

        ModuleTrace {
            x: TracePol::from_ptr(ptr, std::mem::size_of::<F>(), num_rows),
            q: TracePol::from_ptr(ptr_x.add::<F>(), std::mem::size_of::<F>(), num_rows),
            x_mod: TracePol::from_ptr(ptr_x.add::<F>(), std::mem::size_of::<F>(), num_rows),
            buffer: Some(buffer),
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * Self::ROW_SIZE).as_mut_ptr() },
            num_rows,
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

        ModuleTrace {
            x: TracePol::from_ptr(ptr, std::mem::size_of::<F>(), num_rows),
            q: TracePol::from_ptr(ptr_x.add::<F>(), std::mem::size_of::<F>(), num_rows),
            x_mod: TracePol::from_ptr(ptr_x.add::<F>(), std::mem::size_of::<F>(), num_rows),
            buffer: None,
            ptr: unsafe { std::slice::from_raw_parts_mut(ptr, num_rows * stride).as_mut_ptr() },
            num_rows,
        }
    }
}