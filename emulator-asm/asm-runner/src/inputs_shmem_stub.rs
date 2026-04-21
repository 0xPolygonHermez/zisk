use std::sync::Arc;

use zisk_common::io::StreamSink;

use crate::ControlShmem;

use anyhow::Result;

pub struct InputsShmemWriter;

impl InputsShmemWriter {
    pub fn new(
        _shm_prefix: &str,
        _sem_prefix: &str,
        _unlock_mapped_memory: bool,
        _control_writer: Arc<ControlShmem>,
    ) -> Result<Self> {
        unreachable!(
            "InputsShmemWriter::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn write_input(&self, _inputs: &[u8]) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::write_input() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn append_input(&self, _inputs: &[u8]) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::append_input() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn reset(&self) {
        unreachable!(
            "InputsShmemWriter::reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamSink for InputsShmemWriter {
    fn submit(&self, _hints: &[u64]) -> anyhow::Result<()> {
        unreachable!(
            "InputsShmemWriter::submit() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    fn reset(&self) {
        unreachable!(
            "InputsShmemWriter::reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}
