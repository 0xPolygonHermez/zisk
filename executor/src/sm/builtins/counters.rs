//! Counters for the built-in SMs.

use mem_common::MemCounters;
use precomp_dma::DmaCounterInputGen;
use sm_arith::ArithCounterInputGen;
use sm_binary::BinaryCounter;
use zisk_core::MemDataSection;

use super::state_machines::{ARITH_POSITION, BINARY_POSITION, DMA_POSITION, MEM_POSITION};

/// Counter slots for the built-in SMs. Each tuple is `(bundle_position, counter)`.
pub struct BuiltinCounters {
    pub mem: (usize, Option<MemCounters>),
    pub binary: (usize, BinaryCounter),
    pub arith: (usize, ArithCounterInputGen),
    pub dma: (usize, DmaCounterInputGen),
}

impl BuiltinCounters {
    /// Builds the builtin counters for the SMs. If `is_asm` is true, the memory counter will not be initialized.
    pub(crate) fn build(is_asm: bool, mem_sections: Option<&dyn MemDataSection>) -> Self {
        let mem = if is_asm {
            None
        } else {
            let mut counter = MemCounters::new();
            if let Some(mem_sections) = mem_sections {
                counter.init_with_mem_sections(mem_sections);
            }
            Some(counter)
        };
        Self {
            mem: (MEM_POSITION, mem),
            binary: (BINARY_POSITION, BinaryCounter::new()),
            arith: (ARITH_POSITION, ArithCounterInputGen::for_counter_phase()),
            dma: (DMA_POSITION, DmaCounterInputGen::for_counter_phase(is_asm)),
        }
    }
}
