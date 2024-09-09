use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct FibonacciSquarePublics {
    pub module: u64,
    pub a: u64,
    pub b: u64,
    pub out: Option<u64>,
}

impl FibonacciSquarePublics {
    pub fn inner(&self) -> (u64, u64, u64, Option<u64>) {
        (self.module, self.a, self.b, self.out)
    }
}

impl From<&[u8]> for FibonacciSquarePublics {
    fn from(input_bytes: &[u8]) -> Self {
        const U64_SIZE: usize = std::mem::size_of::<u64>();
        assert_eq!(input_bytes.len(), U64_SIZE * 4, "Input bytes length must be 4 * size_of::<u64>()");

        FibonacciSquarePublics {
            module: u64::from_le_bytes(input_bytes[0..U64_SIZE].try_into().unwrap()),
            a: u64::from_le_bytes(input_bytes[U64_SIZE..2 * U64_SIZE].try_into().unwrap()),
            b: u64::from_le_bytes(input_bytes[2 * U64_SIZE..3 * U64_SIZE].try_into().unwrap()),
            out: Some(u64::from_le_bytes(input_bytes[3 * U64_SIZE..4 * U64_SIZE].try_into().unwrap())),
        }
    }
}

impl From<FibonacciSquarePublics> for Vec<u8> {
    fn from(val: FibonacciSquarePublics) -> Self {
        let mut bytes = Vec::with_capacity(4 * std::mem::size_of::<u64>());
        bytes.extend_from_slice(&val.module.to_le_bytes());
        bytes.extend_from_slice(&val.a.to_le_bytes());
        bytes.extend_from_slice(&val.b.to_le_bytes());
        bytes.extend_from_slice(&val.out.unwrap_or(0).to_le_bytes());
        bytes
    }
}
