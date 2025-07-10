#[derive(Debug, Default, Clone)]
pub struct MemModuleCheckPoint {
    pub from_addr: u32,
    pub from_skip: u32,
    pub to_addr: u32,
    pub to_count: u32,
    pub count: u32,
}

impl MemModuleCheckPoint {
    pub fn new(from_addr: u32, skip: u32, count: u32) -> Self {
        Self { from_addr, from_skip: skip, to_addr: from_addr, to_count: count, count }
    }
    pub fn init(
        &mut self,
        from_addr: u32,
        skip: u32,
        to_addr: u32,
        to_count: u32,
        count: u32,
    ) -> Self {
        Self { from_addr, from_skip: skip, to_addr, to_count, count }
    }
    pub fn add_rows(&mut self, addr: u32, count: u32) {
        // data is processed by order address, an only one address by chunk/step
        // TODO: assert -> debug_assert
        assert!(addr >= self.to_addr);

        self.count += count;

        if addr == self.to_addr {
            self.to_count += count;
        } else {
            // how address, steps are ordered, if addr ! = self.to_addr means that the new address
            // is greater than the previous one. If we take a new to_addr, we restart the counter,
            // because the previous value of counter refers to previous address
            self.to_addr = addr;
            self.to_count = count;
        }
    }
}
