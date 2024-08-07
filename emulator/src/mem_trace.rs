#[derive(Debug, Clone, Copy)]
pub struct MemTrace {
    pub is_write: bool,
    pub address: u64,
    pub width: u64, // TODO: Ask Jordi, since it is always 8
    pub value: u64,
}

impl MemTrace {
    #[inline(always)]
    pub fn new(is_write: bool, address: u64, width: u64, value: u64) -> Self {
        Self { is_write, address, width, value }
    }
}
