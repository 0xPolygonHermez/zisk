pub struct BinaryInput {
    pub op: u8,
    pub a: u64,
    pub b: u64,
}

impl BinaryInput {
    pub fn new(op: u8, a: u64, b: u64) -> Self {
        Self { op, a, b }
    }
}
