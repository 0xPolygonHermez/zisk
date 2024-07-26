pub struct MemTrace {
    pub is_write: bool,
    pub address: u64,
    pub width: u64, // TODO: Ask Jordi, since it is always 8
    pub value: u64,
}
