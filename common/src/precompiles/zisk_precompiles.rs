use super::MemPrecompileOps;

/// Zisk precompiled
#[derive(Debug, Default, Clone, PartialEq)]
pub enum PrecompiledEmulationMode {
    #[default]
    None,
    GenerateMemReads,
    ConsumeMemReads,
}

/// Zisk precompiled instruction context.
/// Stores the input data (of the size expected by the precompiled components) and the output data.
/// If the precompiled component finds input_data not empty, it should use this data instead of
/// reading it from memory
#[derive(Debug, Default)]
pub struct PrecompiledInstContext {
    /// Precompiled emulation mode
    pub emulation_mode: PrecompiledEmulationMode,
    /// Precompiled input data
    pub data: Vec<u64>,
}

pub trait ZiskPrecompile: Send + Sync {
    fn execute(
        &self,
        a: u64,
        b: u64,
        emulation_mode: PrecompiledEmulationMode,
        mem_ops: MemPrecompileOps,
    ) -> (u64, bool, Vec<u64>);
}

pub struct MemBusHelpers {}

const MEMORY_LOAD_OP: u64 = 1;
const MEMORY_STORE_OP: u64 = 2;

const MEM_STEP_BASE: u64 = 1;
const MAX_MEM_OPS_BY_MAIN_STEP: u64 = 4;

impl MemBusHelpers {
    pub fn mem_aligned_load(addr: u32, step: u64, mem_value: u64) -> [u64; 7] {
        [
            MEMORY_LOAD_OP,
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2,
            8,
            mem_value,
            0,
            0,
        ]
    }
    pub fn mem_aligned_write(addr: u32, step: u64, value: u64) -> [u64; 7] {
        [
            MEMORY_STORE_OP,
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3,
            8,
            0,
            0,
            value,
        ]
    }
}
