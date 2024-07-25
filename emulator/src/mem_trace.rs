pub struct MemTrace {
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}
