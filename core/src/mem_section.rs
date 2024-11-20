/// Memory section data, including a buffer vector, and start and end addresses
pub struct MemSection {
    pub start: u64,
    pub end: u64,
    pub real_end: u64,
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
        MemSection { start: 0, end: 0, real_end: 0, buffer: Vec::new() }
    }
    pub fn to_text(&self) -> String {
        let s = format!(
            "start={:x} real_end={:x} end={:x} diff={:x}={} buffer.len={:x}={}",
            self.start,
            self.real_end,
            self.end,
            self.end - self.start,
            self.end - self.start,
            self.buffer.len(),
            self.buffer.len()
        );
        s
    }
}
