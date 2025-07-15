use crate::{MemInput, MemInstanceInfo};
use proofman_common::AirInstance;

impl MemInput {
    pub fn new(addr: u32, is_write: bool, step: u64, value: u64) -> Self {
        MemInput { addr, is_write, step, value }
    }
}

pub trait MemModule<F: Clone>: Send + Sync {
    fn compute_witness(
        &self,
        mem_ops: &[MemInput],
        mem_instance_info: &MemInstanceInfo,
        trace_buffer: Option<Vec<F>>,
    ) -> AirInstance<F>;
    fn get_addr_range(&self) -> (u32, u32);
}
