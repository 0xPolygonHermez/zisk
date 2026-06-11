//! Non-Linux-x86_64 stub: placeholder type whose methods are unreachable. Off
//! the supported platform these are never exercised, so they carry no docs.
#![allow(missing_docs)]

use std::sync::Arc;

use zisk_common::io::{StreamProcessor, StreamSink};

use crate::ControlShmem;

use anyhow::Result;

pub struct InputsShmemWriter;

impl InputsShmemWriter {
    pub fn new(
        _shm_prefix: &str,
        _unlock_mapped_memory: bool,
        _control_writer: Arc<ControlShmem>,
    ) -> Result<Self> {
        unreachable!(
            "InputsShmemWriter::new() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn bind_semaphores(&self, _sem_prefix: &str) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::bind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn unbind_semaphores(&self) {
        unreachable!(
            "InputsShmemWriter::unbind_semaphores() is not supported on this platform. Only Linux x86_64 is supported."
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

    pub fn signal_reset(&self) -> Result<()> {
        unreachable!(
            "InputsShmemWriter::signal_reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    pub fn reset(&self) {
        unreachable!(
            "InputsShmemWriter::reset() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }
}

impl StreamProcessor for InputsShmemWriter {
    fn process_hints(&self, _data: &[u64], _first_batch: bool) -> anyhow::Result<bool> {
        unreachable!(
            "InputsShmemWriter::process_hints() is not supported on this platform. Only Linux x86_64 is supported."
        );
    }

    fn reset(&self) {
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
