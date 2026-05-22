//! Counters for the built-in SMs and their construction from the bundle.

use crate::error::{ExecutorError, ExecutorResult};
use crate::{StateMachines, StaticSMBundle};
use fields::PrimeField64;
use mem_common::MemCounters;
use precomp_dma::DmaCounterInputGen;
use sm_arith::ArithCounterInputGen;
use sm_binary::BinaryCounter;

use super::state_machines::BuiltinSMs;

/// Counters for the built-in SMs.
pub struct BuiltinCounters {
    /// Memory-related counters.
    pub mem: (usize, Option<MemCounters>),
    /// Binary operation counters.
    pub binary: (usize, BinaryCounter),
    /// Arithmetic operation counters.
    pub arith: (usize, ArithCounterInputGen),
    /// DMA-related counters.
    pub dma: (usize, DmaCounterInputGen),
}

impl BuiltinCounters {
    /// Constructs the built-in counters from the executor's bundle.
    ///
    /// # Errors
    /// Returns [`ExecutorError::BundleComponentMissing`] if any expected
    /// built-in SM is absent from the bundle.
    pub(crate) fn from_bundle<F: PrimeField64>(bundle: &StaticSMBundle<F>) -> ExecutorResult<Self> {
        let is_asm = bundle.is_asm();

        let mut mem = None;
        let mut binary = None;
        let mut arith = None;
        let mut dma = None;

        for (pos, (_, sm)) in bundle.entries().iter().enumerate() {
            if let StateMachines::Builtin(b) = sm {
                match b {
                    BuiltinSMs::RomSM(_) => {}
                    BuiltinSMs::MemSM(sm) => {
                        if mem.is_some() {
                            return Err(ExecutorError::BundleComponentDuplicate { kind: "Mem" });
                        }
                        let counter = if is_asm { None } else { Some(sm.build_mem_counter()) };
                        mem = Some((pos, counter));
                    }
                    BuiltinSMs::BinarySM(sm) => {
                        if binary.is_some() {
                            return Err(ExecutorError::BundleComponentDuplicate { kind: "Binary" });
                        }
                        binary = Some((pos, sm.build_binary_counter()));
                    }
                    BuiltinSMs::ArithSM(sm) => {
                        if arith.is_some() {
                            return Err(ExecutorError::BundleComponentDuplicate { kind: "Arith" });
                        }
                        arith = Some((pos, sm.build_arith_counter()));
                    }
                    BuiltinSMs::DmaManager(sm) => {
                        if dma.is_some() {
                            return Err(ExecutorError::BundleComponentDuplicate { kind: "Dma" });
                        }
                        dma = Some((pos, sm.build_dma_counter(is_asm)));
                    }
                }
            }
        }

        let mem = mem.ok_or(ExecutorError::BundleComponentMissing { kind: "Mem" })?;
        let binary = binary.ok_or(ExecutorError::BundleComponentMissing { kind: "Binary" })?;
        let arith = arith.ok_or(ExecutorError::BundleComponentMissing { kind: "Arith" })?;
        let dma = dma.ok_or(ExecutorError::BundleComponentMissing { kind: "Dma" })?;

        Ok(Self { mem, binary, arith, dma })
    }
}
