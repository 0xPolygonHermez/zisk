pub struct FibonacciVadcopInputs {
    pub a: u64,
    pub b: u64,
    pub module: u64,
}

impl FibonacciVadcopInputs {
    pub fn inner(&self) -> (u64, u64, u64) {
        (self.a, self.b, self.module)
    }
}

impl From<&[u8]> for FibonacciVadcopInputs {
    fn from(input_bytes: &[u8]) -> Self {
        const U64_SIZE: usize = std::mem::size_of::<u64>();
        assert_eq!(input_bytes.len(), U64_SIZE * 3, "Input bytes length must be 3 * size_of::<u64>()");

        FibonacciVadcopInputs {
            a: u64::from_le_bytes(input_bytes[0..U64_SIZE].try_into().unwrap()),
            b: u64::from_le_bytes(input_bytes[U64_SIZE..2 * U64_SIZE].try_into().unwrap()),
            module: u64::from_le_bytes(input_bytes[2 * U64_SIZE..3 * U64_SIZE].try_into().unwrap()),
        }
    }
}
