/// Memory section data, including a buffer (a vector of bytes) and start and end addresses
#[derive(Default)]
pub struct MemSection {
    pub start: u64,
    pub end: u64,
    pub buffer: Vec<u8>,
}
