use std::sync::Arc;

use crate::{MemHelpers, MemInput, MemPreviousSegment, MEM_BYTES};
use proofman_common::{AirInstance, ProofCtx, SetupCtx};
use zisk_core::ZiskRequiredMemory;

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64) -> Self {
        MemInput { addr, is_write, step, value }
    }
    pub fn from(mem_op: &ZiskRequiredMemory) -> Self {
        match mem_op {
            ZiskRequiredMemory::Basic { step, value, address, is_write, width, step_offset } => {
                debug_assert_eq!(*width, MEM_BYTES as u8);
                MemInput {
                    addr: address >> 3,
                    is_write: *is_write,
                    step: MemHelpers::main_step_to_address_step(*step, *step_offset),
                    value: *value,
                }
            }
            ZiskRequiredMemory::Extended { values: _, address: _ } => {
                panic!("MemInput::from() called with an extended instance");
            }
        }
    }
}

pub trait MemModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        segment_id: usize,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
    ) -> AirInstance<F>;

    fn get_addr_ranges(&self) -> Vec<(u32, u32)>;
    fn debug(&self, _pctx: Arc<ProofCtx<F>>, _sctx: Arc<SetupCtx<F>>) {}
}
