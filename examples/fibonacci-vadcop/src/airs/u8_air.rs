use proofman::trace_pol::TracePol;

pub struct U8AirTrace0<F> {
    pub mul: TracePol<F>,
}

impl<F> U8AirTrace0<F> {
    pub fn from_buffer(ptr: &Vec<u8>, num_rows: usize) -> Self {
        Self::from_ptr(ptr.as_ptr(), num_rows, 0)
    }

    /// Constructs a `FibonacciTrace<F>` from a raw pointer, number of rows, and offset.
    ///
    /// # Safety
    /// - The `ptr` must point to valid memory.
    /// - The memory region starting from `ptr` must be properly aligned and sized to hold
    ///   at least `num_rows` elements of type `F`.
    ///
    /// # Parameters
    /// - `ptr`: A mutable raw pointer pointing to the start of the memory region.
    /// - `num_rows`: The number of rows to read from the memory region.
    /// - `offset`: Offset in bytes from `ptr` to the start of the data
    pub fn from_ptr(ptr: *const u8, num_rows: usize, offset: usize) -> Self {
        const OFFSET_MUL: usize = 0;

        let f_size = std::mem::size_of::<F>();
        let row_size = 3 * f_size;

        // Adjust pointer by offset
        let mut ptr = unsafe { ptr.add(offset) as *mut u8 };

        // Create TracePol instances
        ptr = unsafe { ptr.add(f_size * OFFSET_MUL) };
        let mul = TracePol::from_ptr(ptr, row_size, num_rows);

        Self { mul }
    }
}
