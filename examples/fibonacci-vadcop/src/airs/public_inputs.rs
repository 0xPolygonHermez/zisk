pub struct FibonacciVadcopPublicInputs {
    pub module: u64,
    pub a: u64,
    pub b: u64,
    pub out: u64,
}

impl FibonacciVadcopPublicInputs {
    pub fn inner(&self) -> (u64, u64, u64, u64) {
        (self.module, self.a, self.b, self.out)
    }
}

impl From<&[u8]> for FibonacciVadcopPublicInputs {
    fn from(input_bytes: &[u8]) -> Self {
        const U64_SIZE: usize = std::mem::size_of::<u64>();
        assert_eq!(input_bytes.len(), U64_SIZE * 4, "Input bytes length must be 4 * size_of::<u64>()");

        FibonacciVadcopPublicInputs {
            module: u64::from_le_bytes(input_bytes[0..U64_SIZE].try_into().unwrap()),
            a: u64::from_le_bytes(input_bytes[U64_SIZE..2 * U64_SIZE].try_into().unwrap()),
            b: u64::from_le_bytes(input_bytes[2 * U64_SIZE..3 * U64_SIZE].try_into().unwrap()),
            out: u64::from_le_bytes(input_bytes[3 * U64_SIZE..4 * U64_SIZE].try_into().unwrap()),
        }
    }
}
