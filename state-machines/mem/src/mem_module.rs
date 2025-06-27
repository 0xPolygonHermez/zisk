use crate::{MemInput, MemPreviousSegment};
use proofman_common::AirInstance;
use zisk_common::SegmentId;

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64) -> Self {
        MemInput { addr, is_write, step, value }
    }
}

pub trait MemModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        segment_id: SegmentId,
        is_last_segment: bool,
        previous_segment: &MemPreviousSegment,
        trace_buffer: Vec<F>,
    ) -> AirInstance<F>;
    fn get_addr_range(&self) -> (u32, u32);
}
