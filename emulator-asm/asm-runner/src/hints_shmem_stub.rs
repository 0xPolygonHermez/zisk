use crate::{AsmService, ControlShmem};
use anyhow::Result;
use std::sync::Arc;
use zisk_common::io::StreamSink;

pub struct HintsShmem;

impl HintsShmem {
    pub fn new(
        _shm_prefix: &str,
        _unlock_mapped_memory: bool,
        _control_writer: Arc<ControlShmem>,
        _active_services: &[AsmService],
    ) -> Result<Self> {
        unreachable!(
            "HintsShmem::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn bind_semaphores(&self, _sem_prefix: &str) -> Result<()> {
        unreachable!(
            "HintsShmem::bind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn unbind_semaphores(&self) {
        unreachable!(
            "HintsShmem::unbind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
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
