use proofman::trace_pol::TracePol;

pub struct FibonacciSquareTrace0<F> {
    pub a: TracePol<F>,
    pub b: TracePol<F>,
}

impl<F> FibonacciSquareTrace0<F> {
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
        const OFFSET_A: usize = 0;
        const OFFSET_B: usize = 1;

        let f_size = std::mem::size_of::<F>();
        let row_size = 2 * f_size;

        // Adjust pointer by offset
        let mut ptr = unsafe { ptr.add(offset) as *mut u8 };

        // Create TracePol instances
        ptr = unsafe { ptr.add(f_size * OFFSET_A) };
        let a = TracePol::from_ptr(ptr, row_size, num_rows);

        ptr = unsafe { ptr.add(f_size * OFFSET_B) };
        let b = TracePol::from_ptr(ptr, row_size, num_rows);

        Self { a, b }
    }
}
