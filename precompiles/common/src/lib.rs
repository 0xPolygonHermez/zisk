mod goldilocks_constants;

pub use goldilocks_constants::{get_ks, GOLDILOCKS_GEN, GOLDILOCKS_K};

use std::collections::VecDeque;
use zisk_common::{BusId, MEM_BUS_ID};
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
    pub fn mem_aligned_load(
        addr: u32,
        step: u64,
        mem_value: u64,
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        assert!(addr % 8 == 0);
        pending.push_back((
            MEM_BUS_ID,
            vec![
                MEMORY_LOAD_OP,
                addr as u64,
                MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2,
                8,
                mem_value,
                0,
                0,
            ],
        ));
    }
    pub fn mem_aligned_write(
        addr: u32,
        step: u64,
        value: u64,
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        assert!(addr % 8 == 0);
        pending.push_back((
            MEM_BUS_ID,
            vec![
                MEMORY_STORE_OP,
                addr as u64,
                MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3,
                8,
                0,
                0,
                value,
            ],
        ));
    }
    pub fn mem_aligned_op(
        addr: u32,
        step: u64,
        value: u64,
        is_write: bool,
        pending: &mut VecDeque<(BusId, Vec<u64>)>,
    ) {
        pending.push_back((
            MEM_BUS_ID,
            vec![
                if is_write { MEMORY_STORE_OP } else { MEMORY_LOAD_OP },
                addr as u64,
                MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + if is_write { 3 } else { 2 },
                8,
                if is_write { 0 } else { value },
                0,
                if is_write { value } else { 0 },
            ],
        ));
    }
}

pub fn log2(n: usize) -> usize {
    let mut res = 0;
    let mut n = n;
    while n > 1 {
        n >>= 1;
        res += 1;
    }
    res
}
