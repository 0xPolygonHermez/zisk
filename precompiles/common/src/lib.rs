use zisk_core::InstContext;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct PrecompileCode(u16);

impl PrecompileCode {
    pub fn new(value: u16) -> Self {
        PrecompileCode(value)
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl From<u16> for PrecompileCode {
    fn from(value: u16) -> Self {
        PrecompileCode::new(value)
    }
}

impl From<PrecompileCode> for u16 {
    fn from(code: PrecompileCode) -> Self {
        code.value()
    }
}

pub struct PrecompileContext {}

pub trait PrecompileCall: Send + Sync {
    fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)>;
}

pub struct MemBusHelpers {}

const MEMORY_LOAD_OP: u64 = 1;
const MEMORY_STORE_OP: u64 = 2;

const MEM_STEP_BASE: u64 = 1;
const MAX_MEM_OPS_BY_MAIN_STEP: u64 = 4;

impl MemBusHelpers {
    pub fn mem_aligned_load(addr: u32, step: u64, mem_value: u64) -> [u64; 7] {
        [
            MEMORY_LOAD_OP as u64,
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2 as u64,
            8,
            mem_value,
            0,
            0,
        ]
    }
    pub fn mem_aligned_write(addr: u32, step: u64, value: u64) -> [u64; 7] {
        [
            MEMORY_STORE_OP as u64,
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3 as u64,
            8,
            0,
            0,
            value,
        ]
    }
}
