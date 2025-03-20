use super::PrecompiledEmulationMode;

pub struct MemPrecompileOps<'a> {
    emulation_mode: PrecompiledEmulationMode,
    read_reg_fn: Box<dyn Fn(u64) -> u64 + 'a>,
    read_mem_fn: Box<dyn FnMut(u64, bool) -> u64 + 'a>,
    write_mem_fn: Box<dyn Fn(u64, u64) + 'a>,
    consume_mread: Option<Box<dyn FnMut() -> u64 + 'a>>,
}

impl<'a> MemPrecompileOps<'a> {
    pub fn new(
        emulation_mode: PrecompiledEmulationMode,
        read_reg_fn: Box<dyn Fn(u64) -> u64 + 'a>,
        read_mem_fn: Box<dyn FnMut(u64, bool) -> u64 + 'a>,
        write_mem_fn: Box<dyn Fn(u64, u64) + 'a>,
        consume_mread: Option<Box<dyn FnMut() -> u64 + 'a>>,
    ) -> Self {
        Self { emulation_mode, read_reg_fn, read_mem_fn, write_mem_fn, consume_mread }
    }

    pub fn read_reg(&self, reg: u64) -> u64 {
        (self.read_reg_fn)(reg)
    }

    pub fn read_mem(&mut self, address: u64) -> u64 {
        (self.read_mem_fn)(
            address,
            self.emulation_mode == PrecompiledEmulationMode::GenerateMemReads,
        )
    }

    pub fn write_mem(&self, address: u64, data: u64) {
        (self.write_mem_fn)(address, data)
    }

    pub fn consume_mread(&mut self) -> u64 {
        if let Some(get_mem_read) = &mut self.consume_mread {
            get_mem_read()
        } else {
            panic!("get_mem_read not set")
        }
    }
}
