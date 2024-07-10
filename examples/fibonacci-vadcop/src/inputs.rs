pub struct FibonacciVadcopInputs {
    pub a: usize,
    pub b: usize,
    pub module: usize,
}

impl From<&[u8]> for FibonacciVadcopInputs {
    fn from(input_bytes: &[u8]) -> Self {
        const USIZE_SIZE: usize = std::mem::size_of::<usize>();
        assert_eq!(input_bytes.len(), USIZE_SIZE * 3, "Input bytes length must be 3 * size_of::<usize>()");

        FibonacciVadcopInputs {
            a: usize::from_le_bytes(input_bytes[0..USIZE_SIZE].try_into().unwrap()),
            b: usize::from_le_bytes(input_bytes[USIZE_SIZE..2 * USIZE_SIZE].try_into().unwrap()),
            module: usize::from_le_bytes(input_bytes[2 * USIZE_SIZE..3 * USIZE_SIZE].try_into().unwrap()),
        }
    }
}