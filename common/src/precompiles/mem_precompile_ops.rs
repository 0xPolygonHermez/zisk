pub struct MemPrecompileOps<'a> {
    pub get_mem_read: Option<Box<dyn FnMut() -> u64 + 'a>>,
    pub read_reg_fn: Box<dyn Fn(u64) -> u64 + 'a>,
    pub read_mem_fn: Box<dyn FnMut(u64, bool) -> u64 + 'a>,
    pub write_mem_fn: Box<dyn Fn(u64, u64) + 'a>,
}

impl<'a> MemPrecompileOps<'a> {
    pub fn new(
        get_mem_read: Option<Box<dyn FnMut() -> u64 + 'a>>,
        read_reg_fn: Box<dyn Fn(u64) -> u64 + 'a>,
        read_mem_fn: Box<dyn FnMut(u64, bool) -> u64 + 'a>,
        write_mem_fn: Box<dyn Fn(u64, u64) + 'a>,
    ) -> Self {
        Self { get_mem_read, read_reg_fn, read_mem_fn, write_mem_fn }
    }
}
