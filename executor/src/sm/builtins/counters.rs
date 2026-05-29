//! Counters for the built-in SMs.

use fields::PrimeField64;
use mem_common::MemCounters;
use precomp_dma::{DmaCounterInputGen, DmaManager};
use sm_arith::{ArithCounterInputGen, ArithSM};
use sm_binary::{BinaryCounter, BinarySM};
use sm_mem::Mem;
use zisk_common::ComponentPlanBuilder;

use super::state_machines::{ARITH_POSITION, BINARY_POSITION, DMA_POSITION, MEM_POSITION};

/// Counter slots for the built-in SMs. Each tuple is `(bundle_position, counter)`.
pub struct BuiltinCounters {
    pub mem: (usize, Option<MemCounters>),
    pub binary: (usize, BinaryCounter),
    pub arith: (usize, ArithCounterInputGen),
    pub dma: (usize, DmaCounterInputGen),
}

impl BuiltinCounters {
    /// Builds the slots via static dispatch — no SM bundle required.
    pub(crate) fn build<F: PrimeField64>(is_asm: bool) -> Self {
        let mem =
            if is_asm { None } else { Some(<Mem<F> as ComponentPlanBuilder<F>>::counter(is_asm)) };
        Self {
            mem: (MEM_POSITION, mem),
            binary: (BINARY_POSITION, <BinarySM<F> as ComponentPlanBuilder<F>>::counter(is_asm)),
            arith: (ARITH_POSITION, <ArithSM<F> as ComponentPlanBuilder<F>>::counter(is_asm)),
            dma: (DMA_POSITION, <DmaManager<F> as ComponentPlanBuilder<F>>::counter(is_asm)),
        }
    }
}
