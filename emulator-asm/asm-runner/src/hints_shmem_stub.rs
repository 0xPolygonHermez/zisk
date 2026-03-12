use crate::{AsmService, ControlShmem};
use anyhow::Result;
use std::sync::Arc;
use zisk_common::io::StreamSink;

/// HintsShmem struct manages the writing of processed precompile hints to shared memory.
pub struct HintsShmem;

impl HintsShmem {
    pub fn new(
        _base_port: Option<u16>,
        _local_rank: i32,
        _unlock_mapped_memory: bool,
        _control_writer: Arc<ControlShmem>,
        _active_services: &[AsmService],
    ) -> Result<Self> {
        unreachable!(
            "HintsShmem::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn set_active_services(&self, _active_services: &[AsmService]) -> Result<()> {
        unreachable!(
            "HintsShmem::set_active_services() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamSink for HintsShmem {
    fn submit(&self, _processed: &[u64]) -> anyhow::Result<()> {
        unreachable!(
            "HintsShmem::submit() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}
