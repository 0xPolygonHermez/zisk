//! Common utilities and helpers for Zisk precompiles.

mod goldilocks_constants;

pub use goldilocks_constants::{get_ks, GOLDILOCKS_GEN, GOLDILOCKS_K};

use mem_common::MemCounters;
use sm_mem::{MemAlignCollector, MemModuleCollector};
use zisk_common::MEM_BUS_ID;
use zisk_core::InstContext;

/// Represents a precompile operation code.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct PrecompileCode(u16);

impl PrecompileCode {
    /// Creates a new precompile code from a u16 value.
    pub fn new(value: u16) -> Self {
        PrecompileCode(value)
    }

    /// Returns the underlying u16 value of the precompile code.
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

/// Context for precompile execution.
pub struct PrecompileContext {}

/// Trait for implementing precompile calls.
pub trait PrecompileCall: Send + Sync {
    /// Executes the precompile operation with the given opcode and instruction context.
    /// Returns an optional tuple containing the result value and a boolean flag.
    fn execute(&self, opcode: PrecompileCode, ctx: &mut InstContext) -> Option<(u64, bool)>;
}

/// Helper functions for memory bus operations.
pub struct MemBusHelpers {}

/// Memory load operation code.
const MEMORY_LOAD_OP: u64 = 1;
/// Memory store operation code.
const MEMORY_STORE_OP: u64 = 2;

/// Base step for memory operations.
const MEM_STEP_BASE: u64 = 1;
/// Maximum number of memory operations per main step.
const MAX_MEM_OPS_BY_MAIN_STEP: u64 = 4;

/// Trait for processing memory operations - allows static dispatch
pub trait MemProcessor {
    fn process_mem_data(&mut self, data: &[u64; 7]);
    fn skip_addr(&mut self, addr: u32) -> bool;
    fn skip_addr_range(&mut self, addr_from: u32, addr_to: u32) -> bool;
}

/// Collector-based memory mem_processor
pub struct MemCollectorProcessor<'a> {
    pub mem: &'a mut [(usize, MemModuleCollector)],
    pub align: &'a mut [(usize, MemAlignCollector)],
}

impl<'a> MemCollectorProcessor<'a> {
    #[inline(always)]
    pub fn new(
        mem: &'a mut [(usize, MemModuleCollector)],
        align: &'a mut [(usize, MemAlignCollector)],
    ) -> Self {
        Self { mem, align }
    }
}

impl MemProcessor for MemCollectorProcessor<'_> {
    #[inline(always)]
    fn process_mem_data(&mut self, data: &[u64; 7]) {
        for collector in self.mem.iter_mut() {
            collector.1.process_data(&MEM_BUS_ID, data);
        }
        for collector in self.align.iter_mut() {
            collector.1.process_data(&MEM_BUS_ID, data);
        }
    }

    #[inline(always)]
    fn skip_addr(&mut self, addr: u32) -> bool {
        for collector in self.mem.iter_mut() {
            if !collector.1.skip_addr(addr) {
                return false;
            }
        }
        true
    }

    #[inline(always)]
    fn skip_addr_range(&mut self, addr_from: u32, addr_to: u32) -> bool {
        for collector in self.mem.iter_mut() {
            if !collector.1.skip_addr_range(addr_from, addr_to) {
                return false;
            }
        }
        true
    }
}

/// Counter-based memory mem_processor
pub struct MemCounterProcessor<'a> {
    pub counters: Option<&'a mut MemCounters>,
}

impl<'a> MemCounterProcessor<'a> {
    #[inline(always)]
    pub fn new(counters: Option<&'a mut MemCounters>) -> Self {
        Self { counters }
    }
}

impl MemProcessor for MemCounterProcessor<'_> {
    #[inline(always)]
    fn process_mem_data(&mut self, data: &[u64; 7]) {
        if let Some(counters) = &mut self.counters {
            counters.process_data(&MEM_BUS_ID, data);
        }
    }

    fn skip_addr(&mut self, _addr: u32) -> bool {
        false
    }

    fn skip_addr_range(&mut self, _addr_from: u32, _addr_to: u32) -> bool {
        false
    }
}

impl MemBusHelpers {
    /// Generates an aligned memory load operation.
    /// The address must be 8-byte aligned.
    pub fn mem_aligned_load<P: MemProcessor>(
        addr: u32,
        step: u64,
        mem_value: u64,
        mem_processor: &mut P,
    ) {
        debug_assert!(addr % 8 == 0);
        let data: [u64; 7] = [
            MEMORY_LOAD_OP,
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2,
            8,
            mem_value,
            0,
            0,
        ];
        mem_processor.process_mem_data(&data);
    }

    /// Generates an aligned memory write operation.
    /// The address must be 8-byte aligned.
    pub fn mem_aligned_write<P: MemProcessor>(
        addr: u32,
        step: u64,
        value: u64,
        mem_processor: &mut P,
    ) {
        debug_assert!(addr % 8 == 0);
        let data: [u64; 7] = [
            MEMORY_STORE_OP,
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3,
            8,
            0,
            0,
            value,
        ];
        mem_processor.process_mem_data(&data);
    }

    /// Generates an aligned memory operation (load or write).
    /// The address must be 8-byte aligned.
    pub fn mem_aligned_op<P: MemProcessor>(
        addr: u32,
        step: u64,
        value: u64,
        is_write: bool,
        mem_processor: &mut P,
    ) {
        let data: [u64; 7] = [
            if is_write { MEMORY_STORE_OP } else { MEMORY_LOAD_OP },
            addr as u64,
            MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + if is_write { 3 } else { 2 },
            8,
            if is_write { 0 } else { value },
            0,
            if is_write { value } else { 0 },
        ];

        mem_processor.process_mem_data(&data);
    }

    /// Generates multiple aligned memory load operations from a slice of values.
    /// The address must be 8-byte aligned.
    pub fn mem_aligned_load_from_slice<P: MemProcessor>(
        addr: u32,
        step: u64,
        values: &[u64],
        mem_processor: &mut P,
    ) {
        assert!(addr % 8 == 0);
        let mem_step = MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2;
        for (i, &value) in values.iter().enumerate() {
            let data: [u64; 7] =
                [MEMORY_LOAD_OP, (addr as usize + i * 8) as u64, mem_step, 8, value, 0, 0];

            mem_processor.process_mem_data(&data);
        }
    }
    /// Generates multiple aligned memory write operations from a slice of values.
    /// The address must be 8-byte aligned.
    pub fn mem_aligned_write_from_slice<P: MemProcessor>(
        addr: u32,
        step: u64,
        values: &[u64],
        mem_processor: &mut P,
    ) {
        assert!(addr % 8 == 0);
        let mem_step = MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3;
        for (i, &value) in values.iter().enumerate() {
            let data: [u64; 7] =
                [MEMORY_STORE_OP, (addr as usize + i * 8) as u64, mem_step, 8, 0, 0, value];

            mem_processor.process_mem_data(&data);
        }
    }
    /// Generates aligned memory writes from an unaligned read slice using the specified source offset.
    /// The number of writes generated is `values.len() - 1` because the last value is not enough to
    /// create a full 8-byte write. This function is useful to use the same slice of values to generate
    /// first aligned reads and then aligned writes.
    /// The address must be 8-byte aligned.
    pub fn mem_aligned_write_from_read_unaligned_slice<P: MemProcessor>(
        addr: u32,
        step: u64,
        src_offset: u8,
        values: &[u64],
        mem_processor: &mut P,
    ) {
        assert!(addr % 8 == 0);
        let mem_step = MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3;
        let write_count = values.len() - 1;
        for i in 0..write_count {
            let write_value = match src_offset {
                1 => (values[i] >> 8) | (values[i + 1] << 56),
                2 => (values[i] >> 16) | (values[i + 1] << 48),
                3 => (values[i] >> 24) | (values[i + 1] << 40),
                4 => (values[i] >> 32) | (values[i + 1] << 32),
                5 => (values[i] >> 40) | (values[i + 1] << 24),
                6 => (values[i] >> 48) | (values[i + 1] << 16),
                7 => (values[i] >> 56) | (values[i + 1] << 8),
                _ => panic!("invalid src_offset {src_offset} on DmaUnaligned"),
            };
            let data: [u64; 7] =
                [MEMORY_STORE_OP, (addr as usize + i * 8) as u64, mem_step, 8, 0, 0, write_value];

            mem_processor.process_mem_data(&data);
        }
    }

    /// Returns the memory read step for the given step number.
    pub fn get_mem_read_step(step: u64) -> u64 {
        MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 2
    }
    /// Returns the memory write step for the given step number.
    pub fn get_mem_write_step(step: u64) -> u64 {
        MEM_STEP_BASE + MAX_MEM_OPS_BY_MAIN_STEP * step + 3
    }
}

/// Calculates the base-2 logarithm of n (floor).
pub fn log2(n: usize) -> usize {
    let mut res = 0;
    let mut n = n;
    while n > 1 {
        n >>= 1;
        res += 1;
    }
    res
}
