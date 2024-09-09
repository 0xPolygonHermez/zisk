/// Memory section data, including a buffer vector, and start and end addresses
pub struct MemSection {
    pub start: u64,
    pub end: u64,
    pub buffer: Vec<u8>,
}

/// Default constructor for MemSection structure
impl Default for MemSection {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory section structure implementation
impl MemSection {
    /// Memory section constructor
    pub fn new() -> MemSection {
        MemSection { start: 0, end: 0, buffer: Vec::new() }
    }
}
